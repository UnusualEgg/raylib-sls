#![allow(static_mut_refs)]

#[macro_use]
extern crate lazy_static;
use std::mem::MaybeUninit;
mod state;
use raylib::prelude::*;
pub(crate) use state::State;


#[cfg(target_family = "wasm")]
mod emscripten {
    use std::ffi::{c_int, c_void};
    extern "C" {
        fn emscripten_set_main_loop_arg(loop_fn: extern "C" fn(*mut c_void),user_data: *mut c_void, fps: c_int, sim_infinite_loop: c_int);
        fn emscripten_pause_main_loop();
    }
    struct CBMeta<'d,T,F:FnMut(&mut T)> {
        user_fn:F,
        d:&'d mut T,
    }
    impl<'d,T,F:FnMut(&mut T)> CBMeta<'d,T,F> {
        fn call(&mut self) {
            (self.user_fn)(self.d);
        }
    }
    extern "C" fn load<T,F:FnMut(&mut T)>(data:*mut c_void) {
        let m = data as *mut CBMeta<T,F>;
        let meta: &mut CBMeta<T,F> = unsafe {&mut *m};
        meta.call();
    }
    pub fn set_main_loop<T,F:FnMut(&mut T)>(loop_fn: F,user_data: &mut T, fps:u32, sim_infinite_loop:bool) {
        let mut meta = CBMeta {user_fn:loop_fn,d:user_data};
        unsafe {
            emscripten_set_main_loop_arg(load::<T,F>,&mut meta as *mut CBMeta<T,F> as *mut c_void,fps as c_int, sim_infinite_loop as c_int); 
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
fn draw_loop(state:&mut State) {
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
    //unsafe {&mut STATE}.write(State::new());
    let mut s = State::new();
    #[cfg(target_family = "wasm")]
    emscripten::set_main_loop(draw_loop,&mut s, 0, true);

    #[cfg(not(target_family = "wasm"))]
    {
        while !unsafe{SHOULD_EXIT} {
            draw_loop(&mut s);
        }
    }
    //drop STATE
    // unsafe {
    //     STATE.assume_init_drop();
    // }
    println!("end of main!");
}
