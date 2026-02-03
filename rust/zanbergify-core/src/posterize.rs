/// Map enhanced grayscale + alpha to 3-color RGB output.
///
/// Color mapping:
///
/// - alpha <= 10 -> color_bg
/// - gray < thresh_low -> color_bg
/// - thresh_low <= gray < thresh_high -> color_midtone
/// - gray >= thresh_high -> color_highlight

/// A set of three colors for posterization.
#[derive(Debug, Clone, Copy)]
pub struct ColorPalette {
    pub bg: [u8; 3],
    pub midtone: [u8; 3],
    pub highlight: [u8; 3],
}

impl ColorPalette {
    pub const fn new(bg: [u8; 3], midtone: [u8; 3], highlight: [u8; 3]) -> Self {
        Self {
            bg,
            midtone,
            highlight,
        }
    }

    /// Parse a hex color string like "#FF20AB" or "FF20AB" into [r, g, b].
    pub fn parse_hex(s: &str) -> Result<[u8; 3], String> {
        let s = s.strip_prefix('#').unwrap_or(s);
        if s.len() != 6 {
            return Err(format!("Invalid hex color '{}': expected 6 hex digits", s));
        }
        let r =
            u8::from_str_radix(&s[0..2], 16).map_err(|_| format!("Invalid hex color '{}'", s))?;
        let g =
            u8::from_str_radix(&s[2..4], 16).map_err(|_| format!("Invalid hex color '{}'", s))?;
        let b =
            u8::from_str_radix(&s[4..6], 16).map_err(|_| format!("Invalid hex color '{}'", s))?;
        Ok([r, g, b])
    }
}

// Original palette (matching Python)
pub const PALETTE_ORIGINAL: ColorPalette = ColorPalette::new(
    [0, 0, 0],      // black
    [255, 20, 147], // deep pink / magenta
    [255, 215, 0],  // gold / yellow
);

// Dark burgundy with white highlights
pub const PALETTE_BURGUNDY: ColorPalette = ColorPalette::new(
    [0, 0, 0],       // black
    [114, 5, 70],    // #720546 - dark magenta/burgundy
    [255, 255, 255], // white
);

// Deep burgundy duo with teal complement
pub const PALETTE_BURGUNDY_TEAL: ColorPalette = ColorPalette::new(
    [88, 4, 55],   // #580437 - darkest burgundy
    [114, 5, 70],  // #720546 - burgundy
    [0, 210, 190], // #00D2BE - teal complement
);

// Burgundy with warm gold
pub const PALETTE_BURGUNDY_GOLD: ColorPalette = ColorPalette::new(
    [0, 0, 0],      // black
    [88, 4, 55],    // #580437 - deep burgundy
    [255, 200, 50], // #FFC832 - warm gold
);

// Monochrome burgundy: dark bg, mid burgundy, light rose
pub const PALETTE_ROSE: ColorPalette = ColorPalette::new(
    [88, 4, 55],     // #580437 - deep burgundy
    [180, 30, 100],  // #B41E64 - rose
    [255, 220, 230], // #FFDCE6 - light pink
);

// Cyan and magenta (print-inspired)
pub const PALETTE_CMYK: ColorPalette = ColorPalette::new(
    [0, 0, 0],     // black
    [0, 180, 220], // #00B4DC - cyan
    [230, 0, 120], // #E60078 - magenta
);

pub fn named_palette(name: &str) -> Option<ColorPalette> {
    match name {
        "original" => Some(PALETTE_ORIGINAL),
        "burgundy" => Some(PALETTE_BURGUNDY),
        "burgundy_teal" => Some(PALETTE_BURGUNDY_TEAL),
        "burgundy_gold" => Some(PALETTE_BURGUNDY_GOLD),
        "rose" => Some(PALETTE_ROSE),
        "cmyk" => Some(PALETTE_CMYK),
        _ => None,
    }
}

pub fn all_palette_names() -> &'static [&'static str] {
    &[
        "original",
        "burgundy",
        "burgundy_teal",
        "burgundy_gold",
        "rose",
        "cmyk",
    ]
}

pub fn posterize(
    gray: &[u8],
    alpha: &[u8],
    width: u32,
    height: u32,
    thresh_low: u8,
    thresh_high: u8,
    palette: &ColorPalette,
) -> Vec<u8> {
    let len = (width * height) as usize;
    debug_assert_eq!(gray.len(), len);
    debug_assert_eq!(alpha.len(), len);

    let mut rgb = vec![0u8; len * 3];

    for i in 0..len {
        let color = if alpha[i] <= 10 {
            &palette.bg
        } else if gray[i] < thresh_low {
            &palette.bg
        } else if gray[i] < thresh_high {
            &palette.midtone
        } else {
            &palette.highlight
        };
        rgb[i * 3] = color[0];
        rgb[i * 3 + 1] = color[1];
        rgb[i * 3 + 2] = color[2];
    }

    rgb
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_PALETTE: ColorPalette = PALETTE_ORIGINAL;

    #[test]
    fn test_transparent_is_black() {
        let gray = vec![200u8];
        let alpha = vec![5u8];
        let result = posterize(&gray, &alpha, 1, 1, 70, 150, &TEST_PALETTE);
        assert_eq!(&result[..3], &TEST_PALETTE.bg);
    }

    #[test]
    fn test_shadow_is_black() {
        let gray = vec![50u8];
        let alpha = vec![255u8];
        let result = posterize(&gray, &alpha, 1, 1, 70, 150, &TEST_PALETTE);
        assert_eq!(&result[..3], &TEST_PALETTE.bg);
    }

    #[test]
    fn test_midtone_is_magenta() {
        let gray = vec![100u8];
        let alpha = vec![255u8];
        let result = posterize(&gray, &alpha, 1, 1, 70, 150, &TEST_PALETTE);
        assert_eq!(&result[..3], &TEST_PALETTE.midtone);
    }

    #[test]
    fn test_highlight_is_yellow() {
        let gray = vec![200u8];
        let alpha = vec![255u8];
        let result = posterize(&gray, &alpha, 1, 1, 70, 150, &TEST_PALETTE);
        assert_eq!(&result[..3], &TEST_PALETTE.highlight);
    }

    #[test]
    fn test_boundary_thresh_low() {
        let alpha = vec![255u8; 2];
        let gray = vec![70u8, 69u8];
        let result = posterize(&gray, &alpha, 2, 1, 70, 150, &TEST_PALETTE);
        assert_eq!(&result[0..3], &TEST_PALETTE.midtone);
        assert_eq!(&result[3..6], &TEST_PALETTE.bg);
    }

    #[test]
    fn test_boundary_thresh_high() {
        let alpha = vec![255u8; 2];
        let gray = vec![150u8, 149u8];
        let result = posterize(&gray, &alpha, 2, 1, 70, 150, &TEST_PALETTE);
        assert_eq!(&result[0..3], &TEST_PALETTE.highlight);
        assert_eq!(&result[3..6], &TEST_PALETTE.midtone);
    }

    #[test]
    fn test_parse_hex() {
        assert_eq!(ColorPalette::parse_hex("#720546").unwrap(), [114, 5, 70]);
        assert_eq!(ColorPalette::parse_hex("580437").unwrap(), [88, 4, 55]);
        assert_eq!(ColorPalette::parse_hex("#FFFFFF").unwrap(), [255, 255, 255]);
        assert!(ColorPalette::parse_hex("ZZZ").is_err());
    }

    #[test]
    fn test_custom_palette() {
        let palette = ColorPalette::new([10, 20, 30], [40, 50, 60], [70, 80, 90]);
        let gray = vec![100u8];
        let alpha = vec![255u8];
        let result = posterize(&gray, &alpha, 1, 1, 70, 150, &palette);
        assert_eq!(&result[..3], &[40, 50, 60]);
    }
}
