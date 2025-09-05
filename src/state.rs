use std::collections::HashMap;
use std::mem::ManuallyDrop;
use std::sync::atomic::AtomicBool;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use std::sync::{Arc, LazyLock, Mutex, RwLock};

use raylib::prelude::*;
use raylib::core::math::Vector2;
use raylib::{camera::Camera2D, color::Color, ffi::Gesture, RaylibHandle};
use slslib::sls::{self, Circuit, NodeType, ID};

fn max<T: PartialOrd>(n1: T, n2: T) -> T {
    std::cmp::max_by(n1, n2, |a, b| {
        a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
    })
}
enum ZoomStyle {
    Point1,
    Mid,
}
struct Settings {
    zoom_style: ZoomStyle,
}
//all indexes
struct CompInput {
    in_pin: usize,
    other_pin: usize,
    other_comp: usize,
}
pub struct State {
    pub rl: raylib::core::RaylibHandle,
    pub t: raylib::RaylibThread,
    circuit:  sls::Circuit,
    cam: Camera2D,
    last: Option<Vector2>,
    pointer_on_button:bool,
    comp_labels: Vec<String>,
    in_pin_pos: Vec<Vec<Vector2>>,
    out_pin_pos: Vec<Vec<Vector2>>,
    comp_inputs: Vec<Vec<CompInput>>,
    drag_start:Option<Vector2>,
    initial_distance: f32,
    initial_zoom: f32,
    initial_origin: Vector2,
    settings: Settings,
    tick_rate:f64,
    begin:Instant,
    run_times: usize,
}
const BUTTON_SIZE: f32 = 50.0;
const COMP_SIZE: f32 = 50.0;
const MIN_IC_COMP_SIZE: f32 = 52.0;
const MIN_OUTER_PADDING: f32 = PIN_SIZE + 5.0;
const PIN_SPACING: f32 = (PIN_SIZE * 2.0) + 2.0;
const PIN_SIZE: f32 = 5.0;
const PIN_COLOR: Color = Color::GRAY;
const PIN_LEN: f32 = PIN_SIZE + 2.0;
const ON_COLOR: Color = Color::GREEN;
const OFF_COLOR: Color = Color::BLACK;
const WIRE_THICKNES: f32 = 2.0;
fn calculate_comp_height(node_type:sls::NodeType,max_pins: usize) -> f32 {
    let height: f32 = (max_pins as f32 * PIN_SPACING) + MIN_OUTER_PADDING;
    max(height, if node_type==NodeType::INTEGRATED_CIRCUIT {MIN_IC_COMP_SIZE}else{COMP_SIZE})
}
//returns pos of first pin on left
//add width to get right pin pos
//add PIN_SPACING to get next pin
fn calculate_pin_height(num_of_pins: usize, comp_height: f32) -> f32 {
    let pins_space = num_of_pins as f32 * PIN_SPACING;
    let real_padding = comp_height - pins_space;
    let skip = real_padding / 2.0;
    skip + (PIN_SIZE)
}
fn print_dyn(n: &Circuit, indent: usize) {
    println!("{:indent$}{} {}", ' ', &n.header.name, &n.has_dynamic);
    for comp in n
        .components
        .iter()
        .filter(|c: &&sls::Component| c.node_type == NodeType::INTEGRATED_CIRCUIT)
    {
        let instance = comp.ic_instance.as_ref().unwrap();
        print_dyn(instance, indent + 1);
    }
}
fn get_comp_size(comp:&sls::Component) -> f32 {
    match comp.node_type {
        NodeType::PULSE_BUTTON|NodeType::TOGGLE_BUTTON => BUTTON_SIZE,
        NodeType::INTEGRATED_CIRCUIT=>MIN_IC_COMP_SIZE,
        _=>COMP_SIZE,
    }
}
impl State {
    pub fn new() -> Self {
        let circ = include_str!("../sls/prog-proc-8-bit.slj");
        let mut n: sls::Circuit = serde_json::from_str(circ).unwrap();
        n.init_circ(None);
        n.tick(true);
        let cam = Camera2D {
            offset: Vector2::new(200.0, 200.0),
            target: Vector2::zero(),
            rotation: 0.0,
            zoom: 1.0,
        };
        //print_dyn(&n, 0);
        // n.has_dynamic = true;
        let (mut rl, t) = raylib::init()
            .size(400, 400)
            .title("Hello World")
            .resizable()
            .build();
        //rl.set_target_fps(30);
        rl.set_exit_key(Some(KeyboardKey::KEY_ESCAPE));
        rl.set_gestures_enabled(
            Gesture::GESTURE_HOLD as u32
                | Gesture::GESTURE_TAP as u32
                | Gesture::GESTURE_PINCH_OUT as u32
                | Gesture::GESTURE_PINCH_IN as u32,
        );
        let mut labels:Vec<String> = Vec::with_capacity(n.components.len());
        for comp in &n.components {
            labels.push(match comp.label.as_ref() {
                Some(l) => l.clone(),
                None => match comp.node_type {
                    NodeType::NOTE => "".to_string(),
                    _ => comp.node_type.to_string(),
                },
            });
        }
        let mut in_pin_pos:Vec<Vec<Vector2>> = Vec::with_capacity(n.components.len());
        let mut out_pin_pos:Vec<Vec<Vector2>> = Vec::with_capacity(n.components.len());
        for comp in &n.components {
            let to_num_in = sls::get_num_inputs(comp);
            let to_num_out = comp.outputs.len();
            let to_height = calculate_comp_height(comp.node_type,max(to_num_in, to_num_out));
            let to_in_y_offset = calculate_pin_height(to_num_in, to_height);
            let to_in_y = comp.y + to_in_y_offset;
            let to_out_y_offset = calculate_pin_height(to_num_out, to_height);
            let to_out_y = comp.y + to_out_y_offset;
            let mut in_pin = Vec::with_capacity(to_num_in);
            for i in 0..to_num_in {
                let pin_pos =
                    Vector2::new(comp.x - PIN_LEN, to_in_y + (PIN_SPACING * i as f32));
                in_pin.push(pin_pos);
            }
            in_pin_pos.push(in_pin);
            let mut out_pin = Vec::with_capacity(to_num_out);
            for i in 0..to_num_out {
                let pin_pos =
                    Vector2::new(comp.x + get_comp_size(comp) + PIN_LEN, to_out_y + (PIN_SPACING * i as f32));
                out_pin.push(pin_pos);
            }
            out_pin_pos.push(out_pin);
        }
        let mut comp_inputs = Vec::with_capacity(n.components.len());
        for comp in &n.components {
            let mut inputs = Vec::with_capacity(comp.inputs.len());
            for input in &comp.inputs {
                inputs.push(CompInput {
                    in_pin: input.in_pin,
                    other_pin: input.other_pin,
                    other_comp: n.components.iter().enumerate().find(|(_,n)|n.get_id()==&input.other_id).unwrap().0
                });
            }
            comp_inputs.push(inputs);
        }
        println!("init done!");
        State {
            rl,
            t,
            circuit: n,
            cam,
            last: None,
            drag_start:None,
            initial_distance: 0.0,
            initial_zoom: 1.0,
            initial_origin: Vector2::zero(),
            settings: Settings { zoom_style: ZoomStyle::Mid },
            pointer_on_button: false,
            comp_labels: labels,
            in_pin_pos,
            out_pin_pos,
            comp_inputs,
            tick_rate: 1.0/10.,
            begin: Instant::now(),
            run_times:0,
        }
    }
    fn update_zoom(&mut self,mouse_pos:Vector2) {
        let rl: &mut RaylibHandle = &mut self.rl;
        let scroll = rl.get_mouse_wheel_move();
        if scroll != 0.0 {
            let world_pos = rl.get_screen_to_world2D(mouse_pos, self.cam);
            self.cam.offset = mouse_pos;
            self.cam.target = world_pos;

            // uses log scaling to provide consistent zoom
            let scale = 0.2 * scroll;
            self.cam.zoom = (self.cam.zoom.ln() + scale).exp().clamp(0.125, 64.0);
        }
        if rl.get_touch_point_count()>=2 {
            let p1 = rl.get_touch_position(0);
            let p2 = rl.get_touch_position(1);
            //average
            let origin = match self.settings.zoom_style {
                ZoomStyle::Mid => (p1+p2).scale_by(0.5),
                ZoomStyle::Point1 => p1,
            };
            let world_mid_before = rl.get_screen_to_world2D(origin, self.cam);

            let current_distance = p1.distance_to(p2);

            if self.initial_distance==0.0 {
                self.initial_distance = current_distance;
                self.initial_zoom = self.cam.zoom;
                self.initial_origin = world_mid_before;
            }
            let zoom_factor = current_distance / self.initial_distance;
            self.cam.zoom = self.initial_zoom * zoom_factor;

            //recenter to origin
            let world_origin_after = rl.get_screen_to_world2D(origin, self.cam);
            let offset = self.initial_origin - world_origin_after;
            self.cam.target+=offset;
        } else {
            self.initial_distance=0.0;
        }
    }
    fn update_drag(&mut self,mouse_pos:Vector2) {
        let rl = &mut self.rl;
        if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
            self.drag_start=Some(mouse_pos);
        }
        if rl.is_mouse_button_released(MouseButton::MOUSE_BUTTON_LEFT) {
            self.drag_start=None;
        }
        if let Some(drag) = self.drag_start.as_mut() {
            let delta = *drag-mouse_pos;
            self.cam.target += delta.scale_by(1.0/self.cam.zoom);
            *drag = mouse_pos;
        }
    }
    pub fn update(&mut self) {
        let dur = Duration::from_secs_f64(1./(30.));
        let mut count:usize=0;
        while self.begin.elapsed()<dur{
            // while self.begin.elapsed().as_secs_f64()<self.tick_rate {
            self.circuit.tick(false);
            count+=1;
            // }
        }
        self.run_times=count;
        self.begin=Instant::now();

        if self.rl.is_key_pressed(KeyboardKey::KEY_F) {
            self.rl.toggle_fullscreen();
            if !self.rl.is_window_fullscreen() {
                self.rl.set_window_size(400, 400);
            }
        }
        if self.rl.is_window_resized() {
            self.cam.offset.x = self.rl.get_render_width() as f32 / 2.0;
            self.cam.offset.y = self.rl.get_render_height() as f32 / 2.0;
        }
        let mouse_pos = self.rl.get_mouse_position();

        
        if self.rl.is_gesture_detected(Gesture::GESTURE_TAP) {
            let current = self.rl.get_screen_to_world2D(
                self.rl.get_mouse_position(),
                self.cam,
            );
            for i in 0..self.circuit.inputs.len() {
                let i = self.circuit.inputs[i];
                let comp = &mut self.circuit.components[i];
                let comp_rect = raylib::math::Rectangle::new(comp.x, comp.y, BUTTON_SIZE, BUTTON_SIZE);
                if comp_rect.check_collision_point_rec(current) {
                    if comp.node_type == NodeType::PULSE_BUTTON {
                        comp.outputs[0] = true;
                        self.pointer_on_button = true;
                        self.circuit.comps_changed=true;
                    } else if comp.node_type == NodeType::TOGGLE_BUTTON {
                        comp.outputs[0] = !comp.outputs[0];
                        self.pointer_on_button = true;
                        self.circuit.comps_changed=true;
                    }
                }
            }

            self.last=Some(current);
        } else if self.rl.is_mouse_button_up(MouseButton::MOUSE_BUTTON_LEFT) {
            if let Some(last) = self.last {
                let c =  &mut self.circuit;
                for i in 0..c.inputs.len() {
                    let i = c.inputs[i];
                    let comp = &mut c.components[i];
                    let comp_rect = raylib::math::Rectangle::new(comp.x, comp.y, BUTTON_SIZE, BUTTON_SIZE);
                    if comp_rect.check_collision_point_rec(last) {
                        if comp.node_type == NodeType::PULSE_BUTTON {
                            comp.outputs[0] = false;
                            self.pointer_on_button = false;
                            c.comps_changed=true;
                        }
                    }
                }
            }
            self.pointer_on_button=false;
            self.last=None;
        }
        if !self.pointer_on_button {
            self.update_drag(mouse_pos);
            self.update_zoom(mouse_pos);
        }
    }
    pub fn draw(&mut self) {
        const BUTTON_BORDER: f32 = 5.0;
        const LABEL_SIZE: i32 = 12;
        const NOTE_SIZE: i32 = 40;

        let rl = &mut self.rl;
        let t = &self.t;
        let mut draw = rl.begin_drawing(t);
        draw.clear_background(Color::RAYWHITE);
        let screen_rect = {
            let screen_corner = Vector2::new(draw.get_render_width() as f32, draw.get_render_height() as f32);
            let corner = draw.get_screen_to_world2D(screen_corner, self.cam);
            let tl = draw.get_screen_to_world2D(Vector2::zero(), self.cam);
            Rectangle::new(tl.x, tl.y, corner.x-tl.x, corner.y-tl.y)
        };
        {
            let mut draw = draw.begin_mode2D(self.cam);
            draw.draw_circle(0, 0, 50.0, Color::PINK);
            let c = &self.circuit;
            for (comp_i, comp) in c.components.iter().enumerate() {
                let to_num_in = sls::get_num_inputs(comp);
                let to_num_out = comp.outputs.len();
                let to_height = calculate_comp_height(comp.node_type,max(to_num_in, to_num_out));
                let label = &self.comp_labels[comp_i];
                let size = draw.measure_text(label, LABEL_SIZE);
                //TODO actually make sure label is below ic's
                draw.draw_text(
                    label,
                    (comp.x + (MIN_IC_COMP_SIZE / 2.0)) as i32 - (size / 2),
                    (comp.y + to_height) as i32,
                    LABEL_SIZE,
                    Color::BLACK,
                );
                //draw wires
                //wire goes *from* one component *to* this component
                //plus components have *in*put pins and *out*put pins
                let to_in_y_offset = calculate_pin_height(to_num_in, to_height);
                let to_in_y = comp.y + to_in_y_offset;
                let to_out_y_offset = calculate_pin_height(to_num_out, to_height);
                let to_out_y = comp.y + to_out_y_offset;
                for i in 0..to_num_in {
                    let pin_pos =
                        Vector2::new(comp.x - PIN_LEN, to_in_y + (PIN_SPACING * i as f32));
                    let pin_pos_line = Vector2::new(
                        comp.x - PIN_LEN + PIN_SIZE,
                        to_in_y + (PIN_SPACING * i as f32),
                    );
                    let comp_pos = Vector2::new(comp.x, to_in_y + (PIN_SPACING * i as f32));
                    if screen_rect.check_collision_point_rec(pin_pos) {
                        draw.draw_line_ex(pin_pos_line, comp_pos, WIRE_THICKNES, PIN_COLOR);
                        draw.draw_circle_lines_v(pin_pos, PIN_SIZE, PIN_COLOR);
                    }
                }
                if comp.node_type != NodeType::LIGHT_BULB {
                    for i in 0..to_num_out {
                        let comp_size = get_comp_size(comp);
                        let comp_right = comp.x+comp_size;
                        let pin_pos = Vector2::new(
                            comp_right + PIN_LEN,
                            to_out_y + (PIN_SPACING * i as f32),
                        );
                        let pin_pos_line = Vector2::new(
                            (comp_right + PIN_LEN) - PIN_SIZE,
                            to_out_y + (PIN_SPACING * i as f32),
                        );
                        let comp_pos = Vector2::new(
                            comp_right,
                            to_out_y + (PIN_SPACING * i as f32),
                        );
                        if screen_rect.check_collision_point_rec(pin_pos) {
                            draw.draw_line_ex(pin_pos_line, comp_pos, WIRE_THICKNES, PIN_COLOR);
                            draw.draw_circle_lines_v(pin_pos, PIN_SIZE, PIN_COLOR);
                        }
                    }
                }
                for input in &self.comp_inputs[comp_i] {
                    let on = match comp.input_states.get(input.in_pin){Some(s)=>*s,None=>panic!("tried to get {} of {:#?}",&input.in_pin,comp)};
                    let color = if on { ON_COLOR } else { OFF_COLOR };
                    let p1 = self.out_pin_pos[input.other_comp][input.other_pin];
                    let p2 = self.in_pin_pos[comp_i][input.in_pin];
                    if screen_rect.check_collision_point_rec(p1)||screen_rect.check_collision_point_rec(p2) {
                        draw.draw_line_ex(p1, p2, WIRE_THICKNES, color);
                    }
                }
                match comp.node_type {
                    sls::NodeType::LIGHT_BULB => {
                        let b: bool = comp.outputs[0];
                        let color = if b { Color::LIGHTGREEN } else { Color::BLACK };
                        const LIGHT_RADIUS: f32 = COMP_SIZE / 2.0;
                        let pos = Vector2::new(comp.x + LIGHT_RADIUS, comp.y + LIGHT_RADIUS);
                        draw.draw_circle_v(pos, LIGHT_RADIUS, Color::GRAY);
                        draw.draw_circle_v(pos, LIGHT_RADIUS - BUTTON_BORDER, color);
                    }
                    sls::NodeType::PULSE_BUTTON => {
                        let b: bool = comp.outputs[0];
                        let color = if b { Color::DARKRED } else { Color::RED };
                        const BUTTON_RADIUS: f32 = BUTTON_SIZE / 2.0;
                        let pos = Vector2::new(comp.x + BUTTON_RADIUS, comp.y + BUTTON_RADIUS);
                        draw.draw_circle_v(pos, BUTTON_RADIUS, Color::ORANGE);
                        draw.draw_circle_v(pos, BUTTON_RADIUS - BUTTON_BORDER, color);
                    }
                    sls::NodeType::TOGGLE_BUTTON => {
                        let b: bool = comp.outputs[0];
                        let color = if b { Color::DARKRED } else { Color::RED };
                        let pos = Vector2::new(comp.x, comp.y);
                        draw.draw_rectangle_v(
                            pos,
                            Vector2::new(BUTTON_SIZE, BUTTON_SIZE),
                            Color::ORANGE,
                        );
                        let pos = Vector2::new(comp.x + BUTTON_BORDER, comp.y + BUTTON_BORDER);
                        draw.draw_rectangle_v(
                            pos,
                            Vector2::new(
                                BUTTON_SIZE - (BUTTON_BORDER * 2.0),
                                BUTTON_SIZE - (BUTTON_BORDER * 2.0),
                            ),
                            color,
                        );
                    }
                    sls::NodeType::NOTE => {
                        let text: &str = comp.text.as_ref().expect("text field of NODE");
                        draw.draw_text(text, comp.x as i32, comp.y as i32, NOTE_SIZE, Color::BLACK);
                    }
                    sls::NodeType::SEVEN_SEGMENT_DISPLAY => {
                        const SEGMENT_LENGTH:f32 = 40.0;
                        const SEGMENT_WIDTH:f32 = 5.0;

                        let pos = Vector2::new(comp.x, comp.y);
                        draw.draw_rectangle_v(pos, Vector2::new(COMP_SIZE, to_height), Color::BLACK);
                        let x1 = comp.x+(COMP_SIZE/2.0)-(SEGMENT_LENGTH/2.0);
                        let x2 = comp.x+(COMP_SIZE/2.0)+(SEGMENT_LENGTH/2.0);
                        let y1 = comp.y;
                        let y2 = comp.y + (to_height/2.0);
                        let y3 = comp.y + to_height;
                        let points = [
                            Vector2::new(x1,y1),//0
                            Vector2::new(x2,y1),//1
                            Vector2::new(x1,y2),//2
                            Vector2::new(x2,y2),//3
                            Vector2::new(x1,y3),//4
                            Vector2::new(x2,y3),//5
                        ];
                        //a
                        draw.draw_line_ex(
                            points[0],
                            points[1], 
                            SEGMENT_WIDTH, if comp.input_states[0] {ON_COLOR} else {OFF_COLOR});
                        //b
                        draw.draw_line_ex(
                            points[1],
                            points[3], 
                            SEGMENT_WIDTH, if comp.input_states[1] {ON_COLOR} else {OFF_COLOR});
                        //c
                        draw.draw_line_ex(
                            points[3],
                            points[5], 
                            SEGMENT_WIDTH, if comp.input_states[2] {ON_COLOR} else {OFF_COLOR});
                        //d
                        draw.draw_line_ex(
                            points[4],
                            points[5], 
                            SEGMENT_WIDTH, if comp.input_states[3] {ON_COLOR} else {OFF_COLOR});
                        //e
                        draw.draw_line_ex(
                            points[2],
                            points[4], 
                            SEGMENT_WIDTH, if comp.input_states[4] {ON_COLOR} else {OFF_COLOR});
                        //f
                        draw.draw_line_ex(
                            points[0],
                            points[2], 
                            SEGMENT_WIDTH, if comp.input_states[5] {ON_COLOR} else {OFF_COLOR});
                        //g
                        draw.draw_line_ex(
                            points[2],
                            points[3],
                            SEGMENT_WIDTH, if comp.input_states[6] {ON_COLOR} else {OFF_COLOR});

                    }
                    NodeType::SEVEN_SEGMENT_DISPLAY_DECODER => {}
                    _ => {
                        let color = if let Some(ic) = &comp.ic_instance {
                            if ic.comps_changed {Color::VIOLET}else{Color::DARKRED}
                        } else {
                            Color::GRAY
                        };
                        let pos = Vector2::new(comp.x, comp.y);
                        draw.draw_rectangle_v(pos, Vector2::new(COMP_SIZE, to_height), color);
                    }
                }
                
            }
        }
        draw.draw_fps(0, 0);
        draw.draw_text(&format!("ran {} times",self.run_times), 0, 10, 12, Color::BLACK);
        if draw.get_touch_point_count()>=2 {
            let tp1 = draw.get_touch_position(0);
            let tp2 = draw.get_touch_position(1);
            //draw.draw_circle_v(tp1, 5.0, Color::PINK);
            //draw.draw_circle_v(tp2, 5.0, Color::PURPLE);
            let mid = (tp1+tp2).scale_by(0.5);
            draw.draw_circle_v(mid, 2.0, Color::BLUE);
        }
        const BOUNDS_W:f32 = 0.4;
        const BOUNDS_H:f32 = 0.1;
        let w = draw.get_render_width() as f32;
        let h = draw.get_render_height() as f32;
        let bw = BOUNDS_W*w;
        let bh = BOUNDS_H*h;
        let tick_speed = self.tick_rate as f32;
        draw.gui_label(Rectangle::new(w-bw, h-bh-bh, bw, bh), &format!("{}",1.0/tick_speed));
        // draw.gui_slider(Rectangle::new(w-bw, h-bh, bw, bh),"Tick Speed","",&mut tick_speed, 0.005,0.010);
        self.tick_rate = tick_speed as f64;

        //draw.gui_window_box(
        //    Rectangle::new(0.0, 0.0, 70.0, 70.0),
        //    Some(c"To the window to the wall"),
        //);
        //draw.gui_label(Rectangle::new(0.0, 50.0, 50.0, 20.0), Some(c"ewwo world"));
    }
}
