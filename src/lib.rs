#![recursion_limit = "512"]

mod app;
mod utils;

use wasm_bindgen::prelude::*;
// This is the entry point for the web app
#[wasm_bindgen]
pub fn run_app() {
    utils::set_panic_hook();
    /*web_logger::init();
    yew::start_app::<app::App>();
    Ok(())*/
    yew::start_app::<app::App>();
}
