/// CLAHE (Contrast Limited Adaptive Histogram Equalization)
///
/// Matches OpenCV's implementation:
/// - Divide image into tile_size x tile_size grid
/// - Per-tile histogram with clip limit and iterative redistribution
/// - Bilinear interpolation between tile LUTs

pub fn clahe(gray: &[u8], width: u32, height: u32, clip_limit: f64, tile_size: u32) -> Vec<u8> {
    let w = width as usize;
    let h = height as usize;

    // Number of tiles in each direction
    let tiles_x = tile_size as usize;
    let tiles_y = tile_size as usize;

    // Tile dimensions (may not divide evenly - handle with care)
    let tile_w = w.div_ceil(tiles_x);
    let tile_h = h.div_ceil(tiles_y);

    // Compute LUT for each tile
    let mut luts = vec![vec![0u8; 256]; tiles_x * tiles_y];

    for ty in 0..tiles_y {
        for tx in 0..tiles_x {
            let x_start = tx * tile_w;
            let y_start = ty * tile_h;
            let x_end = (x_start + tile_w).min(w);
            let y_end = (y_start + tile_h).min(h);
            let tile_pixels = (x_end - x_start) * (y_end - y_start);

            // Build histogram
            let mut hist = [0u32; 256];
            for y in y_start..y_end {
                for x in x_start..x_end {
                    hist[gray[y * w + x] as usize] += 1;
                }
            }

            // Clip histogram and redistribute (iterative, matching OpenCV)
            let actual_clip = ((clip_limit * tile_pixels as f64) / 256.0).max(1.0) as u32;
            clip_histogram(&mut hist, actual_clip);

            // Build CDF and create lookup table
            let mut cdf = [0u32; 256];
            cdf[0] = hist[0];
            for i in 1..256 {
                cdf[i] = cdf[i - 1] + hist[i];
            }

            let total = cdf[255];
            if total == 0 {
                for (i, lut_val) in luts[ty * tiles_x + tx].iter_mut().enumerate() {
                    *lut_val = i as u8;
                }
            } else {
                for i in 0..256 {
                    // Scale CDF to [0, 255] range
                    // Match OpenCV: (cdf[i] * 255 + total/2) / total, but use scale factor
                    let val = ((cdf[i] as f64 * 255.0) / total as f64 + 0.5) as u32;
                    luts[ty * tiles_x + tx][i] = val.min(255) as u8;
                }
            }
        }
    }

    // Apply bilinear interpolation between tile LUTs
    let mut output = vec![0u8; w * h];

    for y in 0..h {
        for x in 0..w {
            // Find position relative to tile centers
            // Tile centers are at (tx * tile_w + tile_w/2, ty * tile_h + tile_h/2)
            let fx = (x as f64 - tile_w as f64 / 2.0) / tile_w as f64;
            let fy = (y as f64 - tile_h as f64 / 2.0) / tile_h as f64;

            let tx0 = (fx.floor() as i32).clamp(0, tiles_x as i32 - 1) as usize;
            let ty0 = (fy.floor() as i32).clamp(0, tiles_y as i32 - 1) as usize;
            let tx1 = (tx0 + 1).min(tiles_x - 1);
            let ty1 = (ty0 + 1).min(tiles_y - 1);

            let ax = (fx - tx0 as f64).clamp(0.0, 1.0);
            let ay = (fy - ty0 as f64).clamp(0.0, 1.0);

            let val = gray[y * w + x] as usize;

            let tl = luts[ty0 * tiles_x + tx0][val] as f64;
            let tr = luts[ty0 * tiles_x + tx1][val] as f64;
            let bl = luts[ty1 * tiles_x + tx0][val] as f64;
            let br = luts[ty1 * tiles_x + tx1][val] as f64;

            let top = tl * (1.0 - ax) + tr * ax;
            let bottom = bl * (1.0 - ax) + br * ax;
            let result = top * (1.0 - ay) + bottom * ay;

            output[y * w + x] = (result + 0.5).clamp(0.0, 255.0) as u8;
        }
    }

    output
}

/// Clip histogram bins at `limit` and iteratively redistribute excess counts.
/// Matches OpenCV's iterative redistribution approach.
fn clip_histogram(hist: &mut [u32; 256], limit: u32) {
    // Iterative clipping - cap iterations to avoid infinite loops
    for _ in 0..256 {
        let mut excess = 0u32;
        for h in hist.iter_mut() {
            if *h > limit {
                excess += *h - limit;
                *h = limit;
            }
        }

        if excess == 0 {
            break;
        }

        // Distribute excess: add at most up to limit per bin
        let avg_inc = excess / 256;
        let remainder = (excess % 256) as usize;

        if avg_inc > 0 {
            for i in 0..256 {
                hist[i] = (hist[i] + avg_inc).min(limit);
            }
        }

        // Distribute remainder one per bin, respecting limit
        let mut distributed = 0usize;
        for h in hist.iter_mut() {
            if distributed >= remainder {
                break;
            }
            if *h < limit {
                *h += 1;
                distributed += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uniform_image() {
        // A uniform image should remain roughly uniform after CLAHE
        let gray = vec![128u8; 64 * 64];
        let result = clahe(&gray, 64, 64, 4.0, 8);
        // All pixels should map to the same value
        let first = result[0];
        assert!(result
            .iter()
            .all(|&v| (v as i32 - first as i32).unsigned_abs() <= 1));
    }

    #[test]
    fn test_clip_histogram_basic() {
        let mut hist = [0u32; 256];
        hist[0] = 1000;
        hist[1] = 500;
        clip_histogram(&mut hist, 100);
        assert!(hist.iter().all(|&v| v <= 100));
    }
}
