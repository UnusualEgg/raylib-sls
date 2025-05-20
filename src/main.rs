#![allow(static_mut_refs)]

mod state;
pub(crate) use state::State;
use std::cell::LazyCell;

//#[cfg(target_family = "wasm")]
//mod emscripten;


fn main() {
    println!("Hewroo world :3!");
    std::env::set_var("RUST_BACKTRACE", "full");
    //unsafe {&mut STATE}.write(State::new());
    //static s:std::cell::LazyCell<State> = std::cell::LazyCell::new(||State::new());
    //let mut s:LazyCell<State> = LazyCell::new(||State::new());
    #[cfg(target_family = "wasm")]
    emscripten_functions::emscripten::set_main_loop_with_arg(|state|{
        state.update();
        state.draw();
    },State::new(), 0, true);
    println!("uhoh");

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
