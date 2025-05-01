use raylib::core::math::Vector2;
use raylib::prelude::*;
use raylib::{camera::Camera2D, color::Color, ffi::Gesture, RaylibHandle};

pub struct State {
    pub rl: Option<raylib::core::RaylibHandle>,
    pub t: Option<raylib::RaylibThread>,
    circuit: sls::Circuit,
    cam: Camera2D,
    last: Option<Vector2>,
    last_two: Option<(Vector2,Vector2)>,
}
use slslib::sls::{self, NodeType};
const COMP_SIZE: f32 = 50.0;
impl State {
    pub fn new() -> Self {
        let (rl, t) = raylib::init()
            .size(400, 400)
            .title("Hello World")
            .resizable()
            .build();

        let circ = include_str!("../OR");
        let mut n: sls::Circuit = serde_json::from_str(circ).unwrap();
        n.init_circ(None);
        let cam = Camera2D {
            offset: Vector2::zero(),
            target: Vector2::zero(),
            rotation: 0.0,
            zoom: 1.0,
        };
        State {
            rl: Some(rl),
            t: Some(t),
            circuit: n,
            cam,
            last: None,
            last_two: None,
        }
    }
    pub fn update(&mut self) {
        let rl: &RaylibHandle = self.rl.as_ref().unwrap();
        let gesture = rl.get_gesture_detected();
        if rl.get_touch_point_count() == 1 {
            let current = rl.get_screen_to_world2D(rl.get_touch_position(0),self.cam);
            if let Some(last) = self.last {
                let last = rl.get_screen_to_world2D(last, self.cam);
                self.cam.target.x -= current.x - last.x;
                self.cam.target.y -= current.y - last.y;
                for i in self.circuit.inputs.iter() {
                    let comp = &mut self.circuit.components[*i];
                    if current.x >= comp.x
                        && current.x <= comp.x + COMP_SIZE
                        && current.y >= comp.y
                        && current.y <= comp.y + COMP_SIZE
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
                        && current.x <= comp.x + COMP_SIZE
                        && current.y >= comp.y
                        && current.y <= comp.y + COMP_SIZE
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
                        && last.x <= comp.x + COMP_SIZE
                        && last.y >= comp.y
                        && last.y <= comp.y + COMP_SIZE
                    {
                        if comp.node_type == NodeType::PULSE_BUTTON {
                            comp.next_outputs[0] = false;
                        }
                    }
                }
            }
        }
        if let Some((last1,last2)) = self.last_two {
            if rl.get_touch_point_count() ==2 {
                let current1 = (rl.get_touch_position(0));
                let current2 = (rl.get_touch_position(1));
                //TODO drag
                
            }
        } 
        match gesture {
            Gesture::GESTURE_DRAG => {}
            _ => (),
        }
        match rl.get_touch_point_count() {
            1 => {
                self.last = Some(rl.get_touch_position(0));
            }
            _ => {
                self.last = None;
            }
        }
        match rl.get_touch_point_count() {
            2 => {
                self.last_two =
                Some((rl.get_touch_position(0),rl.get_touch_position(1)));
            }
            _ => {
                self.last_two = None;
            }
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
                match comp.node_type {
                    sls::NodeType::LIGHT_BULB => {
                        let b: bool = comp.outputs.try_borrow().unwrap()[0];
                        let color = if b { Color::GREEN } else { Color::BLACK };
                        let pos = Vector2::new(comp.x + 25.0, comp.y + 25.0);
                        draw.draw_circle_v(pos, 25.0, Color::GRAY);
                        draw.draw_circle_v(pos, 20.0, color);
                    }
                    sls::NodeType::PULSE_BUTTON => {
                        let b: bool = comp.outputs.try_borrow().unwrap()[0];
                        let color = if b { Color::DARKRED } else { Color::RED };
                        let pos = Vector2::new(comp.x + 25.0, comp.y + 25.0);
                        draw.draw_circle_v(pos, 25.0, Color::ORANGE);
                        draw.draw_circle_v(pos, 20.0, color);
                    }
                    sls::NodeType::TOGGLE_BUTTON => {
                        let b: bool = comp.outputs.try_borrow().unwrap()[0];
                        let color = if b { Color::DARKRED } else { Color::RED };
                        let pos = Vector2::new(comp.x, comp.y);
                        draw.draw_rectangle_v(pos, Vector2::new(50.0, 50.0), Color::ORANGE);
                        let pos = Vector2::new(comp.x+5.0, comp.y+5.0);
                        draw.draw_rectangle_v(pos, Vector2::new(50.0-5.0, 50.0-5.0), color);
                    }
                    _ => {
                        let pos = Vector2::new(comp.x, comp.y);
                        draw.draw_rectangle_v(pos, Vector2::new(50.0, 50.0), Color::GRAY);
                    }
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
