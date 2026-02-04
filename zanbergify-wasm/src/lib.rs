use wasm_bindgen::prelude::*;

mod params;
mod processor;
mod utils;

pub use params::{ColorPalette, DetailedParams};
pub use processor::ZanbergifyProcessor;

/// Initialize the WASM module (sets up panic hook).
#[wasm_bindgen(start)]
pub fn init() {
    utils::set_panic_hook();
}
