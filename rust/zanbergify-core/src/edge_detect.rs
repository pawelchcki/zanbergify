/// Sobel edge detection with thresholding, dilation, and overlay.
///
/// Used by the comic pipeline to produce bold outlines on posterized images.

/// Reflect index at borders using BORDER_REFLECT_101 (dcb|abcdefg|fed).
#[inline]
fn reflect101(idx: i32, size: i32) -> i32 {
    if idx < 0 {
        -idx
    } else if idx >= size {
        2 * (size - 1) - idx
    } else {
        idx
    }
}

/// Compute Sobel gradient magnitude for a grayscale image.
///
/// Returns a Vec<u8> of the same size with magnitudes normalized to 0-255.
pub fn sobel_magnitude(gray: &[u8], width: u32, height: u32) -> Vec<u8> {
    let w = width as i32;
    let h = height as i32;
    let len = (width * height) as usize;
    debug_assert_eq!(gray.len(), len);

    // First pass: compute raw magnitudes as u16 and track max
    let mut raw = vec![0u16; len];
    let mut max_val: u16 = 0;

    for y in 0..h {
        for x in 0..w {
            let ym1 = reflect101(y - 1, h);
            let yp1 = reflect101(y + 1, h);
            let xm1 = reflect101(x - 1, w);
            let xp1 = reflect101(x + 1, w);

            // Sobel X kernel:  -1 0 +1
            //                  -2 0 +2
            //                  -1 0 +1
            let tl = gray[(ym1 * w + xm1) as usize] as i32;
            let ml = gray[(y * w + xm1) as usize] as i32;
            let bl = gray[(yp1 * w + xm1) as usize] as i32;
            let tr = gray[(ym1 * w + xp1) as usize] as i32;
            let mr = gray[(y * w + xp1) as usize] as i32;
            let br = gray[(yp1 * w + xp1) as usize] as i32;

            let gx = -tl + tr - 2 * ml + 2 * mr - bl + br;

            // Sobel Y kernel:  -1 -2 -1
            //                   0  0  0
            //                  +1 +2 +1
            let tc = gray[(ym1 * w + x) as usize] as i32;
            let bc = gray[(yp1 * w + x) as usize] as i32;

            let gy = -tl - 2 * tc - tr + bl + 2 * bc + br;

            // Approximate magnitude (avoid sqrt for speed)
            let mag = (gx.unsigned_abs() + gy.unsigned_abs()).min(u16::MAX as u32) as u16;
            raw[(y * w + x) as usize] = mag;
            if mag > max_val {
                max_val = mag;
            }
        }
    }

    // Normalize to 0-255
    if max_val == 0 {
        return vec![0u8; len];
    }

    raw.iter()
        .map(|&v| ((v as u32 * 255) / max_val as u32).min(255) as u8)
        .collect()
}

/// Threshold edge magnitudes and optionally dilate to produce a binary edge map.
///
/// - `threshold`: magnitudes >= threshold become edges
/// - `edge_width`: dilation radius (0 = no dilation, 1 = 3x3, 2 = 5x5, etc.)
pub fn threshold_and_dilate(
    magnitudes: &[u8],
    threshold: u8,
    edge_width: u8,
    width: u32,
    height: u32,
) -> Vec<bool> {
    let w = width as usize;
    let h = height as usize;
    let len = w * h;
    debug_assert_eq!(magnitudes.len(), len);

    // Initial threshold
    let thresholded: Vec<bool> = magnitudes.iter().map(|&v| v >= threshold).collect();

    if edge_width <= 1 {
        return thresholded;
    }

    // Dilate: any pixel within `edge_width - 1` of an edge pixel becomes an edge
    let radius = (edge_width - 1) as i32;
    let mut dilated = vec![false; len];

    for y in 0..h as i32 {
        for x in 0..w as i32 {
            if thresholded[(y as usize) * w + (x as usize)] {
                for dy in -radius..=radius {
                    for dx in -radius..=radius {
                        let nx = x + dx;
                        let ny = y + dy;
                        if nx >= 0 && nx < w as i32 && ny >= 0 && ny < h as i32 {
                            dilated[(ny as usize) * w + (nx as usize)] = true;
                        }
                    }
                }
            }
        }
    }

    dilated
}

/// Composite edge pixels onto an RGB buffer.
///
/// Where `edges[i]` is true, the pixel is blended toward `edge_color` by `alpha` (0.0-1.0).
/// An alpha of 1.0 fully replaces the pixel with the edge color.
pub fn overlay_edges(rgb: &mut [u8], edges: &[bool], alpha: f32, edge_color: [u8; 3]) {
    let pixel_count = edges.len();
    debug_assert_eq!(rgb.len(), pixel_count * 3);

    let a = alpha.clamp(0.0, 1.0);
    let inv_a = 1.0 - a;

    for i in 0..pixel_count {
        if edges[i] {
            let idx = i * 3;
            rgb[idx] = (edge_color[0] as f32 * a + rgb[idx] as f32 * inv_a + 0.5) as u8;
            rgb[idx + 1] = (edge_color[1] as f32 * a + rgb[idx + 1] as f32 * inv_a + 0.5) as u8;
            rgb[idx + 2] = (edge_color[2] as f32 * a + rgb[idx + 2] as f32 * inv_a + 0.5) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reflect101() {
        assert_eq!(reflect101(-1, 5), 1);
        assert_eq!(reflect101(0, 5), 0);
        assert_eq!(reflect101(4, 5), 4);
        assert_eq!(reflect101(5, 5), 3);
    }

    #[test]
    fn test_uniform_no_edges() {
        let gray = vec![128u8; 16];
        let mag = sobel_magnitude(&gray, 4, 4);
        assert!(mag.iter().all(|&v| v == 0));
    }

    #[test]
    fn test_sobel_detects_vertical_edge() {
        // Left half dark, right half bright
        #[rustfmt::skip]
        let gray = vec![
            0, 0, 255, 255,
            0, 0, 255, 255,
            0, 0, 255, 255,
            0, 0, 255, 255,
        ];
        let mag = sobel_magnitude(&gray, 4, 4);
        // Pixels at the boundary (columns 1 and 2) should have high magnitude
        assert!(mag[1] > 0 || mag[2] > 0);
    }

    #[test]
    fn test_threshold_basic() {
        let magnitudes = vec![0, 50, 100, 200, 30, 255];
        let edges = threshold_and_dilate(&magnitudes, 100, 1, 6, 1);
        assert_eq!(edges, vec![false, false, true, true, false, true]);
    }

    #[test]
    fn test_dilation_expands() {
        // 5x1 strip with single edge at center
        let magnitudes = vec![0, 0, 255, 0, 0];
        let edges_no_dilate = threshold_and_dilate(&magnitudes, 100, 1, 5, 1);
        assert_eq!(edges_no_dilate, vec![false, false, true, false, false]);

        let edges_dilated = threshold_and_dilate(&magnitudes, 100, 2, 5, 1);
        assert_eq!(edges_dilated, vec![false, true, true, true, false]);
    }

    #[test]
    fn test_overlay_edges_replaces() {
        let mut rgb = vec![255, 0, 0, 0, 255, 0]; // red, green
        let edges = vec![true, false];
        overlay_edges(&mut rgb, &edges, 1.0, [0, 0, 0]);
        assert_eq!(&rgb[0..3], &[0, 0, 0]); // replaced with black
        assert_eq!(&rgb[3..6], &[0, 255, 0]); // untouched
    }
}
