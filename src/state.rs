use std::collections::HashMap;
use std::time::Instant;

use raylib::core::math::Vector2;
use raylib::prelude::*;
use raylib::{camera::Camera2D, color::Color, ffi::Gesture, RaylibHandle};
use slslib::sls::{self, NodeType, ID};

fn max<T: PartialOrd>(n1: T, n2: T) -> T {
    std::cmp::max_by(n1, n2, |a, b| {
        a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
    })
}

pub struct State {
    pub rl: Option<raylib::core::RaylibHandle>,
    pub t: Option<raylib::RaylibThread>,
    circuit: sls::Circuit,
    cam: Camera2D,
    last: Option<Vector2>,
    last_two: Option<(Vector2, Vector2)>,
    comp_indexes: HashMap<ID, usize>, //components index
    pinch: Option<Vector2>,
}
const MIN_COMP_SIZE: f32 = 50.0;
const MIN_OUTER_PADDING: f32 = PIN_SIZE + 5.0;
const PIN_SPACING: f32 = (PIN_SIZE * 2.0) + 2.0;
const PIN_SIZE: f32 = 5.0;
const PIN_COLOR: Color = Color::GRAY;
const PIN_LEN: f32 = PIN_SIZE + 2.0;
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
impl State {
    pub fn new() -> Self {
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

        let circ = include_str!("../sls/prog-proc-8-bit.slj");
        let mut n: sls::Circuit = serde_json::from_str(circ).unwrap();
        n.init_circ(None);
        let cam = Camera2D {
            offset: Vector2::new(200.0, 200.0),
            target: Vector2::zero(),
            rotation: 0.0,
            zoom: 1.0,
        };
        //calculate all positions of circuits
        let mut positions = HashMap::with_capacity(n.components.len());
        for (i, comp) in n.components.iter().enumerate() {
            positions.insert(comp.get_id().clone(), i);
        }
        State {
            rl: Some(rl),
            t: Some(t),
            circuit: n,
            cam,
            last: None,
            last_two: None,
            comp_indexes: positions,
            pinch: None,
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
        if scroll != 0.0 {
            let mouse_pos = rl.get_mouse_position();
            let world_pos = rl.get_screen_to_world2D(mouse_pos, self.cam);
            self.cam.offset = mouse_pos;
            self.cam.target = world_pos;

            // uses log scaling to provide consistent zoom
            let scale = 0.2 * scroll;
            self.cam.zoom = (self.cam.zoom.ln() + scale).exp().clamp(0.125, 64.0);
        }
        let touch_point_count = if mouse { 1 } else { rl.get_touch_point_count() };
        if touch_point_count == 1 {
            let current = rl.get_screen_to_world2D(
                if mouse {
                    rl.get_mouse_position()
                } else {
                    rl.get_touch_position(0)
                },
                self.cam,
            );
            if let Some(last) = self.last {
                let last = rl.get_screen_to_world2D(last, self.cam);
                self.cam.target.x -= current.x - last.x;
                self.cam.target.y -= current.y - last.y;
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
            } else {
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
        } else {
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
        if rl.is_gesture_detected(Gesture::GESTURE_PINCH_IN)
            || rl.is_gesture_detected(Gesture::GESTURE_PINCH_OUT)
        {
            let pinch = rl.get_gesture_pinch_vector();
            if let Some(last) = self.pinch {
                let diff = pinch.length() - last.length();
                //let middle_x = rl.get_touch_x() + pinch.x;
                self.cam.zoom += diff;
            }
            self.pinch = Some(pinch);
        } else {
            self.pinch = None;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_F) {
            rl.toggle_fullscreen();
        }
        self.circuit.tick();
    }
    pub fn draw(&mut self) {
        let rl = self.rl.as_mut().unwrap();
        let t = self.t.as_ref().unwrap();
        let mut draw = rl.begin_drawing(t);
        draw.clear_background(Color::RAYWHITE);
        draw.draw_fps(0, 0);
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
                        let pos = Vector2::new(comp.x + 25.0, comp.y + 25.0);
                        draw.draw_circle_v(pos, 25.0, Color::ORANGE);
                        draw.draw_circle_v(pos, 20.0, color);
                    }
                    sls::NodeType::TOGGLE_BUTTON => {
                        let b: bool = comp.outputs[0];
                        let color = if b { Color::DARKRED } else { Color::RED };
                        let pos = Vector2::new(comp.x, comp.y);
                        draw.draw_rectangle_v(pos, Vector2::new(50.0, 50.0), Color::ORANGE);
                        let pos = Vector2::new(comp.x + 5.0, comp.y + 5.0);
                        draw.draw_rectangle_v(pos, Vector2::new(50.0 - 5.0, 50.0 - 5.0), color);
                    }
                    _ => {
                        let color = if let Some(ic) = &comp.ic_instance {
                            match ic.header.color{
                                Some(c) => {
                                    Color::new(c.r, c.g, c.b, 255)
                                }
                                None => {
                                    Color::GRAY
                                }
                            } 
                        } else {Color::GRAY};
                        let pos = Vector2::new(comp.x, comp.y);
                        draw.draw_rectangle_v(
                            pos,
                            Vector2::new(MIN_COMP_SIZE, to_height),
                            color,
                        );
                        if let Some(label) = &comp.label {
                            let size = draw.measure_text(label, 12);
                            draw.draw_text(
                                label,
                                (comp.x + (MIN_COMP_SIZE / 2.0)) as i32 - (size / 2),
                                (comp.y + to_height) as i32,
                                12,
                                Color::BLACK,
                            );
                        }
                    }
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
                    let comp_pos = Vector2::new(comp.x, to_in_y + (PIN_SPACING * i as f32));
                    draw.draw_line_v(pin_pos, comp_pos, PIN_COLOR);
                    draw.draw_circle_lines_v(pin_pos, PIN_SIZE, PIN_COLOR);
                }
                if comp.node_type!=NodeType::LIGHT_BULB {
                    for i in 0..to_num_out {
                        let pin_pos = Vector2::new(
                            comp.x + PIN_LEN + MIN_COMP_SIZE,
                            to_out_y + (PIN_SPACING * i as f32),
                        );
                        let comp_pos =
                        Vector2::new(comp.x + MIN_COMP_SIZE, to_out_y + (PIN_SPACING * i as f32));
                        draw.draw_line_v(pin_pos, comp_pos, PIN_COLOR);
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
                    draw.draw_line_v(from_vec, to_vec, color);
                }
            }
        }
        //draw.gui_window_box(
        //    Rectangle::new(0.0, 0.0, 70.0, 70.0),
        //    Some(c"To the window to the wall"),
        //);
        //draw.gui_label(Rectangle::new(0.0, 50.0, 50.0, 20.0), Some(c"ewwo world"));
    }
}
unsafe impl Send for State {}
