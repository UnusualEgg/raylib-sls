#[macro_use]
extern crate lazy_static;
use std::ffi::c_int;
use parking_lot::{Mutex, MutexGuard};
mod state;
use raylib::prelude::*;
pub(crate) use state::State;

//use raylib::prelude::*;

#[cfg(target_family = "wasm")]
extern "C" {
    fn emscripten_set_main_loop(loop_fn: extern "C" fn(), fps: c_int, sim_infinite_loop: c_int);
    fn emscripten_pause_main_loop();
}
lazy_static! {
    static ref STATE: Mutex<State> = Mutex::new(State::new());
}
extern "C" fn draw_loop() {
    let mut state: MutexGuard<State> = STATE.lock();
    state.update();
    state.draw();

    let rl: &RaylibHandle = state.rl.as_ref().unwrap();
    if rl.is_key_pressed(KeyboardKey::KEY_RIGHT) {
        println!("Goodbye!");
        state.rl = None;
        state.t = None;
        #[cfg(target_family = "wasm")]
        unsafe {
            emscripten_pause_main_loop();
            return;
        }
        #[cfg(not(target_family = "wasm"))]
        std::process::exit(0);
    }
}

fn main() {
    println!("Hewroo world :3!");
    #[cfg(target_family = "wasm")]
    unsafe {
        emscripten_set_main_loop(draw_loop, 0, 0);
    }

    #[cfg(not(target_family = "wasm"))]
    {
        while !STATE.lock().rl.as_ref().unwrap().window_should_close() {
            draw_loop();
        }
        let mut state = STATE.lock();
        state.rl = None;
        state.t = None;
    }
    println!("end of main!");
}
