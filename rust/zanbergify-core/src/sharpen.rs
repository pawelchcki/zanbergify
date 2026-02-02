/// 3x3 sharpening convolution with kernel:
/// ```text
///  0  -1   0
/// -1   5  -1
///  0  -1   0
/// ```
/// Uses BORDER_REFLECT_101 for edge pixels (matching OpenCV default).

pub fn sharpen(gray: &[u8], width: u32, height: u32) -> Vec<u8> {
    let w = width as i32;
    let h = height as i32;
    let len = (width * height) as usize;
    debug_assert_eq!(gray.len(), len);

    let mut output = vec![0u8; len];

    for y in 0..h {
        for x in 0..w {
            // Reflect border indices (BORDER_REFLECT_101: dcb|abcdefg|fed)
            let ym1 = reflect101(y - 1, h);
            let yp1 = reflect101(y + 1, h);
            let xm1 = reflect101(x - 1, w);
            let xp1 = reflect101(x + 1, w);

            let center = gray[(y * w + x) as usize] as i32;
            let top = gray[(ym1 * w + x) as usize] as i32;
            let bottom = gray[(yp1 * w + x) as usize] as i32;
            let left = gray[(y * w + xm1) as usize] as i32;
            let right = gray[(y * w + xp1) as usize] as i32;

            let val = 5 * center - top - bottom - left - right;
            output[(y * w + x) as usize] = val.clamp(0, 255) as u8;
        }
    }

    output
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reflect101() {
        assert_eq!(reflect101(-1, 5), 1);
        assert_eq!(reflect101(0, 5), 0);
        assert_eq!(reflect101(4, 5), 4);
        assert_eq!(reflect101(5, 5), 3);
        assert_eq!(reflect101(6, 5), 2);
    }

    #[test]
    fn test_uniform_image_unchanged() {
        // A uniform image should remain unchanged after sharpening
        let gray = vec![128u8; 9];
        let result = sharpen(&gray, 3, 3);
        assert_eq!(result, vec![128u8; 9]);
    }

    #[test]
    fn test_sharpen_enhances_edges() {
        // Center pixel brighter than neighbors -> should get even brighter
        #[rustfmt::skip]
        let gray = vec![
            100, 100, 100,
            100, 200, 100,
            100, 100, 100,
        ];
        let result = sharpen(&gray, 3, 3);
        // center: 5*200 - 100 - 100 - 100 - 100 = 600 -> clamped to 255
        assert_eq!(result[4], 255);
    }
}
