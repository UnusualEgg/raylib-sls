use std::collections::HashMap;

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
pub struct State {
    pub rl: Option<raylib::core::RaylibHandle>,
    pub t: Option<raylib::RaylibThread>,
    circuit: sls::Circuit,
    cam: Camera2D,
    last: Option<Vector2>,
    comp_indexes: HashMap<ID, usize>, //components index
    drag_start:Option<Vector2>,
    initial_distance: f32,
    initial_zoom: f32,
    initial_origin: Vector2,
    settings: Settings,
}
const MIN_COMP_SIZE: f32 = 50.0;
const MIN_OUTER_PADDING: f32 = PIN_SIZE + 5.0;
const PIN_SPACING: f32 = (PIN_SIZE * 2.0) + 2.0;
const PIN_SIZE: f32 = 5.0;
const PIN_COLOR: Color = Color::GRAY;
const PIN_LEN: f32 = PIN_SIZE + 2.0;
const WIRE_THICKNES: f32 = 2.0;
fn calculate_comp_height(max_pins: usize) -> f32 {
    let height: f32 = (max_pins as f32 * PIN_SPACING) + MIN_OUTER_PADDING;
    max(height, MIN_COMP_SIZE)
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
        .filter(|c| c.node_type == NodeType::INTEGRATED_CIRCUIT)
    {
        let instance = comp.ic_instance.as_ref().unwrap();
        print_dyn(instance, indent + 1);
    }
}
impl State {
    pub fn new() -> Self {
        let circ = include_str!("../sls/v3/6db39f2f-acb0-4462-9759-eb52e913d996");
        let mut n: sls::Circuit = serde_json::from_str(circ).unwrap();
        n.init_circ(None);
        let cam = Camera2D {
            offset: Vector2::new(200.0, 200.0),
            target: Vector2::zero(),
            rotation: 0.0,
            zoom: 1.0,
        };
        print_dyn(&n, 0);
        n.has_dynamic = true;
        //calculate all positions of circuits
        let mut positions = HashMap::with_capacity(n.components.len());
        for (i, comp) in n.components.iter().enumerate() {
            positions.insert(comp.get_id().clone(), i);
        }
        let (rl, t) = raylib::init()
            .size(400, 400)
            .title("Hello World")
            .resizable()
            .build();
        rl.set_gestures_enabled(
            Gesture::GESTURE_TAP as u32
                | Gesture::GESTURE_PINCH_OUT as u32
                | Gesture::GESTURE_PINCH_IN as u32,
        );
        State {
            rl: Some(rl),
            t: Some(t),
            circuit: n,
            cam,
            last: None,
            comp_indexes: positions,
            drag_start:None,
            initial_distance: 0.0,
            initial_zoom: 1.0,
            initial_origin: Vector2::zero(),
            settings: Settings { zoom_style: ZoomStyle::Mid },
        }
    }
    pub fn update(&mut self) {
        let rl: &mut RaylibHandle = self.rl.as_mut().unwrap();
        if rl.is_window_resized() {
            self.cam.offset.x = rl.get_render_width() as f32 / 2.0;
            self.cam.offset.y = rl.get_render_height() as f32 / 2.0;
        }
        let mouse = rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT);
        let scroll = rl.get_mouse_wheel_move();
        let mouse_pos = rl.get_mouse_position();
        if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
            self.drag_start=Some(mouse_pos);
        }
        if rl.is_mouse_button_released(MouseButton::MOUSE_BUTTON_LEFT) {
            self.drag_start=None;
        }
        if let Some(drag) = self.drag_start.as_mut() {
            let delta = *drag-mouse_pos;
            self.cam.target += delta.scale_by(1.0/self.cam.zoom);
            *drag = rl.get_mouse_position();
        }
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


        let real_count = rl.get_touch_point_count();
        let touch_point_count = if real_count == 0 && mouse {
            1
        } else {
            rl.get_touch_point_count()
        };
        if touch_point_count == 1 {
            let current = rl.get_screen_to_world2D(
                if mouse {
                    rl.get_mouse_position()
                } else {
                    rl.get_touch_position(0)
                },
                self.cam,
            );
            if self.last.is_none() {
                for i in self.circuit.inputs.iter() {
                    let comp = &mut self.circuit.components[*i];
                    if current.x >= comp.x
                        && current.x <= comp.x + MIN_COMP_SIZE
                        && current.y >= comp.y
                        && current.y <= comp.y + MIN_COMP_SIZE
                    {
                        if comp.node_type == NodeType::PULSE_BUTTON {
                            comp.next_outputs[0] = true;
                        }
                    }
                }
                //check for button press
                for i in self.circuit.inputs.iter() {
                    let comp = &mut self.circuit.components[*i];
                    if current.x >= comp.x
                        && current.x <= comp.x + MIN_COMP_SIZE
                        && current.y >= comp.y
                        && current.y <= comp.y + MIN_COMP_SIZE
                    {
                        if comp.node_type == NodeType::TOGGLE_BUTTON {
                            comp.next_outputs[0] = !comp.next_outputs[0];
                        }
                    }
                }
            }
        } else if touch_point_count==0 {
            if let Some(last) = self.last {
                let last = rl.get_screen_to_world2D(last, self.cam);
                for i in self.circuit.inputs.iter() {
                    let comp = &mut self.circuit.components[*i];
                    if last.x >= comp.x
                        && last.x <= comp.x + MIN_COMP_SIZE
                        && last.y >= comp.y
                        && last.y <= comp.y + MIN_COMP_SIZE
                    {
                        if comp.node_type == NodeType::PULSE_BUTTON {
                            comp.next_outputs[0] = false;
                        }
                    }
                }
            }
        }
        match touch_point_count {
            1 => {
                self.last = Some(rl.get_touch_position(0));
            }
            _ => {
                self.last = None;
            }
        }
        if rl.is_key_pressed(KeyboardKey::KEY_F) {
            rl.toggle_fullscreen();
        }
        let t = rl.get_time();
        let tick_sec = 0.001;
        let times = t - (self.circuit.tick_count as f64 * tick_sec);
        for _ in 0..((times / tick_sec) as usize) {
            self.circuit.tick();
        }
    }
    pub fn draw(&mut self) {
        const BUTTON_BORDER: f32 = 5.0;
        const LABEL_SIZE: i32 = 24;
        const NOTE_SIZE: i32 = 48;

        let rl = self.rl.as_mut().unwrap();
        let t = self.t.as_ref().unwrap();
        let mut draw = rl.begin_drawing(t);
        draw.clear_background(Color::RAYWHITE);
        {
            let mut draw = draw.begin_mode2D(self.cam);
            draw.draw_circle(0, 0, 50.0, Color::PINK);
            for comp in &self.circuit.components {
                let to_num_in = comp.input_states.len();
                let to_num_out = comp.outputs.len();
                let to_height = calculate_comp_height(max(to_num_in, to_num_out));
                match comp.node_type {
                    sls::NodeType::LIGHT_BULB => {
                        let b: bool = comp.outputs[0];
                        let color = if b { Color::LIGHTGREEN } else { Color::BLACK };
                        let pos = Vector2::new(comp.x + 25.0, comp.y + 25.0);
                        draw.draw_circle_v(pos, 25.0, Color::GRAY);
                        draw.draw_circle_v(pos, 20.0, color);
                    }
                    sls::NodeType::PULSE_BUTTON => {
                        let b: bool = comp.outputs[0];
                        let color = if b { Color::DARKRED } else { Color::RED };
                        const BUTTON_RADIUS: f32 = MIN_COMP_SIZE / 2.0;
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
                            Vector2::new(MIN_COMP_SIZE, MIN_COMP_SIZE),
                            Color::ORANGE,
                        );
                        let pos = Vector2::new(comp.x + BUTTON_BORDER, comp.y + BUTTON_BORDER);
                        draw.draw_rectangle_v(
                            pos,
                            Vector2::new(
                                MIN_COMP_SIZE - (BUTTON_BORDER * 2.0),
                                MIN_COMP_SIZE - (BUTTON_BORDER * 2.0),
                            ),
                            color,
                        );
                    }
                    sls::NodeType::NOTE => {
                        let text: &str = &comp.text.as_ref().expect("text field of NODE");
                        draw.draw_text(text, comp.x as i32, comp.y as i32, NOTE_SIZE, Color::BLACK);
                    }
                    _ => {
                        let color = if let Some(ic) = &comp.ic_instance {
                            match ic.header.color {
                                Some(c) => Color::new(c.r, c.g, c.b, 255),
                                None => Color::GRAY,
                            }
                        } else {
                            Color::GRAY
                        };
                        let pos = Vector2::new(comp.x, comp.y);
                        draw.draw_rectangle_v(pos, Vector2::new(MIN_COMP_SIZE, to_height), color);
                    }
                }
                if let Some(label) = &comp.label {
                    let size = draw.measure_text(label, LABEL_SIZE);
                    draw.draw_text(
                        label,
                        (comp.x + (MIN_COMP_SIZE / 2.0)) as i32 - (size / 2),
                        (comp.y + to_height) as i32,
                        LABEL_SIZE,
                        Color::BLACK,
                    );
                }
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
                    draw.draw_line_ex(pin_pos_line, comp_pos, WIRE_THICKNES, PIN_COLOR);
                    draw.draw_circle_lines_v(pin_pos, PIN_SIZE, PIN_COLOR);
                }
                if comp.node_type != NodeType::LIGHT_BULB {
                    for i in 0..to_num_out {
                        let pin_pos = Vector2::new(
                            comp.x + PIN_LEN + MIN_COMP_SIZE,
                            to_out_y + (PIN_SPACING * i as f32),
                        );
                        let pin_pos_line = Vector2::new(
                            comp.x + PIN_LEN + MIN_COMP_SIZE - PIN_SIZE,
                            to_out_y + (PIN_SPACING * i as f32),
                        );
                        let comp_pos = Vector2::new(
                            comp.x + MIN_COMP_SIZE,
                            to_out_y + (PIN_SPACING * i as f32),
                        );
                        draw.draw_line_ex(pin_pos_line, comp_pos, WIRE_THICKNES, PIN_COLOR);
                        draw.draw_circle_lines_v(pin_pos, PIN_SIZE, PIN_COLOR);
                    }
                }
                for input in &comp.inputs {
                    let from: &slslib::sls::Component = &self
                        .circuit
                        .components
                        .get(*self.comp_indexes.get(&input.other_id).unwrap())
                        .unwrap();
                    let from_num_in = from.input_states.len();
                    let from_num_out = from.outputs.len();
                    let from_height = calculate_comp_height(max(from_num_in, from_num_out));
                    let from_y_off = calculate_pin_height(from_num_out, from_height);
                    let y_from = from.y + from_y_off + (input.other_pin as f32 * PIN_SPACING);
                    let from_vec = Vector2::new(from.x + MIN_COMP_SIZE + PIN_LEN, y_from);
                    let y_to = comp.y + to_in_y_offset + (input.in_pin as f32 * PIN_SPACING);
                    let to_vec = Vector2::new(comp.x - PIN_LEN, y_to);
                    let on = from.outputs[input.other_pin];
                    const ON_COLOR: Color = Color::GREEN;
                    const OFF_COLOR: Color = Color::BLACK;
                    let color = if on { ON_COLOR } else { OFF_COLOR };
                    draw.draw_line_ex(from_vec, to_vec, WIRE_THICKNES, color);
                }
            }
        }
        draw.draw_fps(0, 0);
        if draw.get_touch_point_count()>=2 {
            let tp1 = draw.get_touch_position(0);
            let tp2 = draw.get_touch_position(1);
            //draw.draw_circle_v(tp1, 5.0, Color::PINK);
            //draw.draw_circle_v(tp2, 5.0, Color::PURPLE);
            let mid = (tp1+tp2).scale_by(0.5);
            draw.draw_circle_v(mid, 2.0, Color::BLUE);
        }

        //draw.gui_window_box(
        //    Rectangle::new(0.0, 0.0, 70.0, 70.0),
        //    Some(c"To the window to the wall"),
        //);
        //draw.gui_label(Rectangle::new(0.0, 50.0, 50.0, 20.0), Some(c"ewwo world"));
    }
}
unsafe impl Send for State {}
