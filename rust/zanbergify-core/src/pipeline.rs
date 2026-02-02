/// Full processing pipeline:
/// load image -> rembg -> grayscale (BT.601) -> CLAHE -> sharpen -> posterize -> save PNG

use image::{DynamicImage, GenericImageView, RgbImage};
use std::path::Path;

use crate::clahe::clahe;
use crate::exif_orientation::apply_exif_orientation;
use crate::posterize::{posterize, ColorPalette, PALETTE_ORIGINAL};
use crate::rembg::{extract_existing_alpha, RembgModel};
use crate::sharpen::sharpen;

/// Processing parameters for the detailed algorithm.
#[derive(Debug, Clone)]
pub struct DetailedParams {
    pub thresh_low: u8,
    pub thresh_high: u8,
    pub clip_limit: f64,
    pub tile_size: u32,
    pub palette: ColorPalette,
}

impl DetailedParams {
    pub fn detailed_standard() -> Self {
        Self {
            thresh_low: 80,
            thresh_high: 160,
            clip_limit: 3.0,
            tile_size: 8,
            palette: PALETTE_ORIGINAL,
        }
    }

    pub fn detailed_strong() -> Self {
        Self {
            thresh_low: 70,
            thresh_high: 150,
            clip_limit: 4.0,
            tile_size: 8,
            palette: PALETTE_ORIGINAL,
        }
    }

    pub fn detailed_fine() -> Self {
        Self {
            thresh_low: 80,
            thresh_high: 160,
            clip_limit: 2.5,
            tile_size: 4,
            palette: PALETTE_ORIGINAL,
        }
    }

    pub fn from_preset(name: &str) -> Option<Self> {
        match name {
            "detailed_standard" => Some(Self::detailed_standard()),
            "detailed_strong" => Some(Self::detailed_strong()),
            "detailed_fine" => Some(Self::detailed_fine()),
            _ => None,
        }
    }

    pub fn all_presets() -> Vec<(&'static str, Self)> {
        vec![
            ("detailed_standard", Self::detailed_standard()),
            ("detailed_strong", Self::detailed_strong()),
            ("detailed_fine", Self::detailed_fine()),
        ]
    }

    /// Return a copy with a different palette applied.
    pub fn with_palette(mut self, palette: ColorPalette) -> Self {
        self.palette = palette;
        self
    }
}

/// Convert RGB to grayscale using BT.601 integer formula (matches OpenCV).
/// gray = (R*4899 + G*9617 + B*1868 + 8192) >> 14
pub fn rgb_to_grayscale(img: &DynamicImage) -> Vec<u8> {
    let (w, h) = img.dimensions();
    let rgb = img.to_rgb8();
    let mut gray = Vec::with_capacity((w * h) as usize);

    for y in 0..h {
        for x in 0..w {
            let pixel = rgb.get_pixel(x, y);
            let r = pixel[0] as u32;
            let g = pixel[1] as u32;
            let b = pixel[2] as u32;
            let val = (r * 4899 + g * 9617 + b * 1868 + 8192) >> 14;
            gray.push(val.min(255) as u8);
        }
    }

    gray
}

/// Extract alpha channel from an image using background removal model or fallback.
pub fn extract_alpha(
    img: &DynamicImage,
    model: Option<&RembgModel>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let (width, height) = img.dimensions();

    if let Some(model) = model {
        let rgba = model.remove_background(img)?;
        let mut alpha = Vec::with_capacity((width * height) as usize);
        for y in 0..height {
            for x in 0..width {
                alpha.push(rgba.get_pixel(x, y)[3]);
            }
        }
        Ok(alpha)
    } else if let Some(existing_alpha) = extract_existing_alpha(img) {
        Ok(existing_alpha)
    } else {
        // No model and no alpha - assume fully opaque
        Ok(vec![255u8; (width * height) as usize])
    }
}

/// Process a single image through the full detailed pipeline.
pub fn process_image(
    img: &DynamicImage,
    model: Option<&RembgModel>,
    params: &DetailedParams,
) -> Result<RgbImage, Box<dyn std::error::Error>> {
    let alpha = extract_alpha(img, model)?;
    process_image_with_alpha(img, &alpha, params)
}

/// Process an image with a pre-computed alpha channel (skips background removal).
pub fn process_image_with_alpha(
    img: &DynamicImage,
    alpha: &[u8],
    params: &DetailedParams,
) -> Result<RgbImage, Box<dyn std::error::Error>> {
    let (width, height) = img.dimensions();

    // Step 1: Convert to grayscale using BT.601
    let gray = rgb_to_grayscale(img);

    // Step 2: Apply CLAHE
    let enhanced = clahe(&gray, width, height, params.clip_limit, params.tile_size);

    // Step 3: Apply sharpening
    let sharpened = sharpen(&enhanced, width, height);

    // Step 4: Posterize to 3 colors
    let rgb_data = posterize(
        &sharpened,
        alpha,
        width,
        height,
        params.thresh_low,
        params.thresh_high,
        &params.palette,
    );

    // Create output image
    RgbImage::from_raw(width, height, rgb_data)
        .ok_or_else(|| "Failed to create output image".into())
}

/// Process and save a single image file.
pub fn process_file(
    input_path: &Path,
    output_path: &Path,
    model: Option<&RembgModel>,
    params: &DetailedParams,
) -> Result<(), Box<dyn std::error::Error>> {
    let img = image::open(input_path)?;
    let img = apply_exif_orientation(img, input_path);
    let result = process_image(&img, model, params)?;
    result.save(output_path)?;
    Ok(())
}
