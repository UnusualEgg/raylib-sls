#![allow(static_mut_refs)]

#[macro_use]
extern crate lazy_static;
use std::mem::MaybeUninit;
mod state;
use raylib::prelude::*;
pub(crate) use state::State;


#[cfg(target_family = "wasm")]
mod emscripten {
    use std::ffi::c_int;
    extern "C" {
        fn emscripten_set_main_loop(loop_fn: extern "C" fn(), fps: c_int, sim_infinite_loop: c_int);
        fn emscripten_pause_main_loop();
    }
    pub fn set_main_loop(loop_fn: extern "C" fn(), fps:u32, sim_infinite_loop:bool) {
        unsafe {
            emscripten_set_main_loop(loop_fn,fps as c_int, sim_infinite_loop as c_int); 
        }
    }
    pub fn pause_main_loop() {
        unsafe {
            emscripten_pause_main_loop();
        }
    }
}

static mut STATE: MaybeUninit<State> = MaybeUninit::uninit();
static mut SHOULD_EXIT: bool = false;
extern "C" fn draw_loop() {
    let state: &mut State = unsafe {STATE.assume_init_mut()};
    state.update();
    state.draw();

    let rl: &RaylibHandle = &state.rl;
    if rl.window_should_close() {
        println!("bye. window_should_close");
        unsafe {
            SHOULD_EXIT=true;
        }
    }
}

fn main() {
    println!("Hewroo world :3!");
    unsafe {&mut STATE}.write(State::new());
    
    #[cfg(target_family = "wasm")]
    emscripten::set_main_loop(draw_loop, 0, false);

    #[cfg(not(target_family = "wasm"))]
    while !unsafe{SHOULD_EXIT} {
        draw_loop();
    }
    //drop STATE
    unsafe {
        STATE.assume_init_drop();
    }
    println!("end of main!");
}
