/// Read EXIF orientation tag and apply rotation/flip to correct image orientation.
///
/// EXIF orientation values:
/// 1 = Normal
/// 2 = Flipped horizontally
/// 3 = Rotated 180°
/// 4 = Flipped vertically
/// 5 = Transposed (flip horizontal + rotate 270° CW)
/// 6 = Rotated 90° CW
/// 7 = Transverse (flip horizontal + rotate 90° CW)
/// 8 = Rotated 270° CW

use image::DynamicImage;
use std::path::Path;

/// Read EXIF orientation from file. Returns None if unreadable or missing.
fn read_exif_orientation(path: &Path) -> Option<u32> {
    let file = std::fs::File::open(path).ok()?;
    let mut bufreader = std::io::BufReader::new(file);
    let exif = exif::Reader::new().read_from_container(&mut bufreader).ok()?;
    let orientation = exif.get_field(exif::Tag::Orientation, exif::In::PRIMARY)?;
    orientation.value.get_uint(0)
}

/// Apply EXIF orientation correction to a loaded image.
/// If orientation cannot be read, returns the image unchanged.
pub fn apply_exif_orientation(img: DynamicImage, path: &Path) -> DynamicImage {
    let orientation = match read_exif_orientation(path) {
        Some(o) if o >= 2 && o <= 8 => o,
        _ => return img,
    };

    match orientation {
        2 => img.fliph(),
        3 => img.rotate180(),
        4 => img.flipv(),
        5 => img.rotate270().fliph(),
        6 => img.rotate90(),
        7 => img.rotate90().fliph(),
        8 => img.rotate270(),
        _ => img,
    }
}
