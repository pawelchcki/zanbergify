use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use image::GenericImageView;
use crate::exif_orientation::apply_exif_orientation_from_bytes;
use crate::pipeline::{AlgorithmParams, extract_alpha_from_image};
use crate::posterize::named_palette;

/// Process image from memory buffer (JPEG/PNG bytes) to raw RGB output.
/// Returns 0 on success, negative error codes on failure.
///
/// Output format: RGB bytes (width * height * 3), width, height written to out params.
///
/// # Safety
/// - input_data must be valid for input_len bytes
/// - output_data must have capacity for at least width*height*3 bytes
/// - preset_name and palette_name must be valid null-terminated UTF-8 strings
#[no_mangle]
pub unsafe extern "C" fn zanbergify_process_bytes(
    input_data: *const u8,
    input_len: usize,
    output_data: *mut u8,
    output_width: *mut u32,
    output_height: *mut u32,
    preset_name: *const c_char,
    palette_name: *const c_char,
) -> i32 {
    // Parse preset name
    let preset_str = match unsafe { CStr::from_ptr(preset_name) }.to_str() {
        Ok(s) => s,
        Err(_) => return -1, // Invalid preset name encoding
    };

    // Parse palette name
    let palette_str = match unsafe { CStr::from_ptr(palette_name) }.to_str() {
        Ok(s) => s,
        Err(_) => return -2, // Invalid palette name encoding
    };

    // Load image from bytes
    let input_slice = unsafe { std::slice::from_raw_parts(input_data, input_len) };
    let img = match image::load_from_memory(input_slice) {
        Ok(img) => img,
        Err(_) => return -3, // Failed to decode image
    };

    // Apply EXIF orientation correction
    let img = apply_exif_orientation_from_bytes(img, input_slice);

    let (width, height) = img.dimensions();

    // Extract alpha channel
    let alpha = extract_alpha_from_image(&img);

    // Get preset parameters
    let mut params = match AlgorithmParams::from_preset(preset_str) {
        Some(p) => p,
        None => return -4, // Unknown preset
    };

    // Apply palette
    if let Some(palette) = named_palette(palette_str) {
        params = params.with_palette(palette);
    } else {
        return -5; // Unknown palette
    }

    // Process image
    let result = match params.process(&img, &alpha) {
        Ok(rgb_img) => rgb_img,
        Err(_) => return -6, // Processing failed
    };

    // Copy RGB data to output buffer
    let rgb_bytes = result.into_raw();
    let output_slice = unsafe { std::slice::from_raw_parts_mut(output_data, rgb_bytes.len()) };
    output_slice.copy_from_slice(&rgb_bytes);

    // Write dimensions
    unsafe {
        *output_width = width;
        *output_height = height;
    }

    0 // Success
}

/// Get required output buffer size for an image.
/// Returns 0 on success, negative on failure.
///
/// # Safety
/// - input_data must be valid for input_len bytes
#[no_mangle]
pub unsafe extern "C" fn zanbergify_get_output_size(
    input_data: *const u8,
    input_len: usize,
    width_out: *mut u32,
    height_out: *mut u32,
) -> i32 {
    let input_slice = unsafe { std::slice::from_raw_parts(input_data, input_len) };
    let img = match image::load_from_memory(input_slice) {
        Ok(img) => img,
        Err(_) => return -3, // Failed to decode image (consistent with zanbergify_process_bytes)
    };

    // Apply EXIF orientation correction to get correct dimensions
    let img = apply_exif_orientation_from_bytes(img, input_slice);

    let (width, height) = img.dimensions();
    unsafe {
        *width_out = width;
        *height_out = height;
    }

    0 // Success
}

/// Get JSON array of available presets.
/// Returns a null-terminated string that must be freed with zanbergify_free_string.
#[no_mangle]
pub extern "C" fn zanbergify_list_presets() -> *const c_char {
    let presets: Vec<&str> = AlgorithmParams::all_presets()
        .iter()
        .map(|(name, _)| *name)
        .collect();

    let json = serde_json::to_string(&presets).unwrap_or_else(|_| "[]".to_string());
    match CString::new(json) {
        Ok(s) => s.into_raw(),
        Err(_) => std::ptr::null(),
    }
}

/// Get JSON array of available palettes.
/// Returns a null-terminated string that must be freed with zanbergify_free_string.
#[no_mangle]
pub extern "C" fn zanbergify_list_palettes() -> *const c_char {
    let palettes: Vec<&str> = crate::posterize::all_palette_names().to_vec();
    let json = serde_json::to_string(&palettes).unwrap_or_else(|_| "[]".to_string());
    match CString::new(json) {
        Ok(s) => s.into_raw(),
        Err(_) => std::ptr::null(),
    }
}

/// Free string returned by list functions.
///
/// # Safety
/// - s must be a pointer returned by zanbergify_list_presets or zanbergify_list_palettes
#[no_mangle]
pub unsafe extern "C" fn zanbergify_free_string(s: *mut c_char) {
    if !s.is_null() {
        drop(unsafe { CString::from_raw(s) });
    }
}
