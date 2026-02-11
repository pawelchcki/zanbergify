use image::ImageFormat;
use razemify_core::exif_orientation::apply_exif_orientation_from_bytes;
use razemify_core::pipeline::{extract_alpha_from_image, process_image_with_alpha};
use std::io::Cursor;
use wasm_bindgen::prelude::*;

use crate::params::{ColorPalette, DetailedParams};

/// Main image processor for razemify.
#[wasm_bindgen]
pub struct RazemifyProcessor;

#[wasm_bindgen]
impl RazemifyProcessor {
    /// Process an image with the given parameters and palette.
    ///
    /// # Arguments
    /// * `image_bytes` - Input image as byte array (PNG, JPEG, WebP, etc.)
    /// * `params` - Processing parameters
    /// * `palette` - Color palette to use
    ///
    /// # Returns
    /// PNG-encoded image bytes
    #[wasm_bindgen(js_name = processImage)]
    pub fn process_image(
        image_bytes: &[u8],
        params: &DetailedParams,
        palette: &ColorPalette,
    ) -> Result<Vec<u8>, JsValue> {
        // Load image from bytes
        let img = image::load_from_memory(image_bytes)
            .map_err(|e| JsValue::from_str(&format!("Failed to load image: {}", e)))?;

        // Apply EXIF orientation correction
        let img = apply_exif_orientation_from_bytes(img, image_bytes);

        // Extract or create alpha channel
        let alpha = extract_alpha_from_image(&img);

        // Apply palette to params
        let params_with_palette = params.inner.clone().with_palette(palette.inner);

        // Process image
        let result = process_image_with_alpha(&img, &alpha, &params_with_palette)
            .map_err(|e| JsValue::from_str(&format!("Failed to process image: {}", e)))?;

        // Encode to PNG
        let mut output = Vec::new();
        let mut cursor = Cursor::new(&mut output);
        result
            .write_to(&mut cursor, ImageFormat::Png)
            .map_err(|e| JsValue::from_str(&format!("Failed to encode PNG: {}", e)))?;

        Ok(output)
    }
}
