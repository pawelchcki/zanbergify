use wasm_bindgen::prelude::*;

mod processor;
mod params;
mod utils;

pub use processor::ZanbergifyProcessor;
pub use params::{DetailedParams, ColorPalette};

/// Initialize the WASM module (sets up panic hook).
#[wasm_bindgen(start)]
pub fn init() {
    utils::set_panic_hook();
}
