use razemify_core::pipeline::DetailedParams as CoreDetailedParams;
use razemify_core::posterize::ColorPalette as CoreColorPalette;
use wasm_bindgen::prelude::*;

/// Processing parameters for the detailed posterization algorithm.
#[wasm_bindgen]
pub struct DetailedParams {
    pub(crate) inner: CoreDetailedParams,
}

#[wasm_bindgen]
impl DetailedParams {
    /// Create parameters with custom values.
    #[wasm_bindgen(constructor)]
    pub fn new(thresh_low: u8, thresh_high: u8, clip_limit: f64, tile_size: u32) -> Self {
        Self {
            inner: CoreDetailedParams {
                thresh_low,
                thresh_high,
                clip_limit,
                tile_size,
                palette: razemify_core::posterize::PALETTE_ORIGINAL,
            },
        }
    }

    /// Standard preset: balanced quality and detail.
    #[wasm_bindgen(js_name = detailedStandard)]
    pub fn detailed_standard() -> Self {
        Self {
            inner: CoreDetailedParams::detailed_standard(),
        }
    }

    /// Strong preset: enhanced contrast and sharpness.
    #[wasm_bindgen(js_name = detailedStrong)]
    pub fn detailed_strong() -> Self {
        Self {
            inner: CoreDetailedParams::detailed_strong(),
        }
    }

    /// Fine preset: fine detail preservation.
    #[wasm_bindgen(js_name = detailedFine)]
    pub fn detailed_fine() -> Self {
        Self {
            inner: CoreDetailedParams::detailed_fine(),
        }
    }
}

/// Color palette for posterization output.
#[wasm_bindgen]
pub struct ColorPalette {
    pub(crate) inner: CoreColorPalette,
}

#[wasm_bindgen]
impl ColorPalette {
    /// Create a custom color palette from hex strings.
    #[wasm_bindgen(constructor)]
    pub fn new(
        bg_hex: &str,
        midtone_hex: &str,
        highlight_hex: &str,
    ) -> Result<ColorPalette, JsValue> {
        let bg = parse_hex_color(bg_hex)?;
        let midtone = parse_hex_color(midtone_hex)?;
        let highlight = parse_hex_color(highlight_hex)?;

        Ok(Self {
            inner: CoreColorPalette {
                bg,
                midtone,
                highlight,
            },
        })
    }

    /// Original palette: burgundy background, cream midtone, orange highlight.
    #[wasm_bindgen(js_name = original)]
    pub fn original() -> Self {
        Self {
            inner: razemify_core::posterize::PALETTE_ORIGINAL,
        }
    }

    /// Burgundy palette (same as original).
    #[wasm_bindgen(js_name = burgundy)]
    pub fn burgundy() -> Self {
        Self {
            inner: razemify_core::posterize::PALETTE_BURGUNDY,
        }
    }

    /// Burgundy with teal complement.
    #[wasm_bindgen(js_name = burgundyTeal)]
    pub fn burgundy_teal() -> Self {
        Self {
            inner: razemify_core::posterize::PALETTE_BURGUNDY_TEAL,
        }
    }

    /// Burgundy with warm gold.
    #[wasm_bindgen(js_name = burgundyGold)]
    pub fn burgundy_gold() -> Self {
        Self {
            inner: razemify_core::posterize::PALETTE_BURGUNDY_GOLD,
        }
    }

    /// Monochrome burgundy: dark burgundy, rose, light pink.
    #[wasm_bindgen(js_name = rose)]
    pub fn rose() -> Self {
        Self {
            inner: razemify_core::posterize::PALETTE_ROSE,
        }
    }

    /// Cyan and magenta (print-inspired).
    #[wasm_bindgen(js_name = cmyk)]
    pub fn cmyk() -> Self {
        Self {
            inner: razemify_core::posterize::PALETTE_CMYK,
        }
    }
}

/// Parse a hex color string (with or without #) into RGB array.
fn parse_hex_color(hex: &str) -> Result<[u8; 3], JsValue> {
    let hex = hex.trim_start_matches('#');

    if hex.len() != 6 {
        return Err(JsValue::from_str("Hex color must be 6 characters (RRGGBB)"));
    }

    let r = u8::from_str_radix(&hex[0..2], 16)
        .map_err(|_| JsValue::from_str("Invalid hex color format"))?;
    let g = u8::from_str_radix(&hex[2..4], 16)
        .map_err(|_| JsValue::from_str("Invalid hex color format"))?;
    let b = u8::from_str_radix(&hex[4..6], 16)
        .map_err(|_| JsValue::from_str("Invalid hex color format"))?;

    Ok([r, g, b])
}
