pub mod runner;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    runner::run();
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use wasm_bindgen::prelude::*;
    use wasm_bindgen_futures::spawn_local;
    use console_error_panic_hook;

    #[wasm_bindgen(start)]
    pub fn main() {
        console_error_panic_hook::set_once();
        spawn_local(async {
            runner::run();
        });
    }
}