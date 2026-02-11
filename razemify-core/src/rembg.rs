/// Background removal using ONNX models via ort.
///
/// Supports multiple model architectures:
/// - U2Net (320x320, simple /255 normalization)
/// - BiRefNet (1024x1024, ImageNet normalization, sigmoid + min-max output)
/// - ISNet (1024x1024, ImageNet normalization, sigmoid output)
use image::{DynamicImage, GenericImageView, GrayImage, RgbaImage};
use ndarray::Array4;
use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use ort::value::Tensor;
use std::path::Path;
use std::sync::Mutex;

/// Supported background removal model architectures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelType {
    U2Net,
    BiRefNet,
    ISNet,
}

impl ModelType {
    /// Input resolution expected by the model.
    pub fn input_size(self) -> u32 {
        match self {
            ModelType::U2Net => 320,
            ModelType::BiRefNet | ModelType::ISNet => 1024,
        }
    }

    /// Parse model type from string.
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "u2net" => Some(ModelType::U2Net),
            "birefnet" => Some(ModelType::BiRefNet),
            "isnet" => Some(ModelType::ISNet),
            _ => None,
        }
    }

    /// Try to detect model type from filename.
    pub fn from_path(path: &Path) -> Option<Self> {
        let stem = path.file_stem()?.to_str()?.to_lowercase();
        if stem.contains("birefnet") {
            Some(ModelType::BiRefNet)
        } else if stem.contains("isnet") {
            Some(ModelType::ISNet)
        } else if stem.contains("u2net") {
            Some(ModelType::U2Net)
        } else {
            None
        }
    }

    pub fn all_names() -> &'static [&'static str] {
        &["u2net", "birefnet", "isnet"]
    }
}

// ImageNet normalization constants
const IMAGENET_MEAN: [f32; 3] = [0.485, 0.456, 0.406];
const IMAGENET_STD: [f32; 3] = [0.229, 0.224, 0.225];

pub struct RembgModel {
    session: Mutex<Session>,
    model_type: ModelType,
}

impl RembgModel {
    /// Load ONNX model from the given path with specified model type.
    pub fn load(
        model_path: &Path,
        model_type: ModelType,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut builder = Session::builder()?.with_intra_threads(4)?;

        // BiRefNet models trigger a shape inference bug in ONNX Runtime 1.23.x
        // (github.com/microsoft/onnxruntime/issues/26261). Using Level1 (basic)
        // optimization works around this by transforming nodes before shape inference.
        if model_type == ModelType::BiRefNet {
            builder = builder.with_optimization_level(GraphOptimizationLevel::Level1)?;
        }

        let session = builder.commit_from_file(model_path)?;

        Ok(Self {
            session: Mutex::new(session),
            model_type,
        })
    }

    /// Remove background from image, returning RGBA image with alpha mask.
    pub fn remove_background(
        &self,
        img: &DynamicImage,
    ) -> Result<RgbaImage, Box<dyn std::error::Error>> {
        let (orig_w, orig_h) = img.dimensions();
        let rgb = img.to_rgb8();
        let input_size = self.model_type.input_size();

        // Resize to model input size
        let resized = image::imageops::resize(
            &rgb,
            input_size,
            input_size,
            image::imageops::FilterType::Lanczos3,
        );

        // Normalize and convert to CHW layout (1, 3, H, W)
        let mut input = Array4::<f32>::zeros((1, 3, input_size as usize, input_size as usize));

        match self.model_type {
            ModelType::U2Net => {
                // Simple /255 normalization
                for y in 0..input_size as usize {
                    for x in 0..input_size as usize {
                        let pixel = resized.get_pixel(x as u32, y as u32);
                        input[[0, 0, y, x]] = pixel[0] as f32 / 255.0;
                        input[[0, 1, y, x]] = pixel[1] as f32 / 255.0;
                        input[[0, 2, y, x]] = pixel[2] as f32 / 255.0;
                    }
                }
            }
            ModelType::BiRefNet | ModelType::ISNet => {
                // ImageNet normalization: (pixel/255 - mean) / std
                for y in 0..input_size as usize {
                    for x in 0..input_size as usize {
                        let pixel = resized.get_pixel(x as u32, y as u32);
                        for c in 0..3 {
                            let val = pixel[c] as f32 / 255.0;
                            input[[0, c, y, x]] = (val - IMAGENET_MEAN[c]) / IMAGENET_STD[c];
                        }
                    }
                }
            }
        }

        // Create tensor from ndarray
        let input_tensor = Tensor::from_array(input)?;

        // Run inference
        let mut session = self
            .session
            .lock()
            .map_err(|e| format!("Failed to lock session: {}", e))?;
        let outputs = session.run(ort::inputs![input_tensor])?;
        let output_value = &outputs[0];

        // Extract as ndarray view
        let output_array = output_value.try_extract_array::<f32>()?;

        // Extract mask - output shape is typically (1, 1, H, W)
        let shape = output_array.shape();
        let mask_h = shape[2];
        let mask_w = shape[3];

        // Process output according to model type
        let mut mask = GrayImage::new(mask_w as u32, mask_h as u32);

        match self.model_type {
            ModelType::U2Net | ModelType::ISNet => {
                // Apply sigmoid, scale to [0,255]
                for y in 0..mask_h {
                    for x in 0..mask_w {
                        let val = output_array[[0, 0, y, x]];
                        let sig = sigmoid(val);
                        mask.put_pixel(x as u32, y as u32, image::Luma([(sig * 255.0) as u8]));
                    }
                }
            }
            ModelType::BiRefNet => {
                // Apply sigmoid, then min-max normalize to [0,1], scale to [0,255]
                let mut sigmoid_values = vec![0.0f32; mask_h * mask_w];
                let mut min_val = f32::MAX;
                let mut max_val = f32::MIN;

                for y in 0..mask_h {
                    for x in 0..mask_w {
                        let val = output_array[[0, 0, y, x]];
                        let sig = sigmoid(val);
                        sigmoid_values[y * mask_w + x] = sig;
                        min_val = min_val.min(sig);
                        max_val = max_val.max(sig);
                    }
                }

                let range = max_val - min_val;
                let range = if range < 1e-6 { 1.0 } else { range };

                for y in 0..mask_h {
                    for x in 0..mask_w {
                        let normalized = (sigmoid_values[y * mask_w + x] - min_val) / range;
                        mask.put_pixel(
                            x as u32,
                            y as u32,
                            image::Luma([(normalized * 255.0) as u8]),
                        );
                    }
                }
            }
        }

        // Resize mask back to original dimensions
        let mask_resized =
            image::imageops::resize(&mask, orig_w, orig_h, image::imageops::FilterType::Lanczos3);

        // Create RGBA image with alpha from mask
        let rgb_orig = img.to_rgb8();
        let mut rgba = RgbaImage::new(orig_w, orig_h);
        for y in 0..orig_h {
            for x in 0..orig_w {
                let rgb_pixel = rgb_orig.get_pixel(x, y);
                let alpha = mask_resized.get_pixel(x, y)[0];
                // Threshold at 0.5 (128) for binary mask
                let alpha_binary = if alpha > 128 { 255 } else { 0 };
                rgba.put_pixel(
                    x,
                    y,
                    image::Rgba([rgb_pixel[0], rgb_pixel[1], rgb_pixel[2], alpha_binary]),
                );
            }
        }

        Ok(rgba)
    }
}

#[inline]
fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

/// Helper function to check for model files in a directory
fn check_directory_for_models(
    dir: &std::path::Path,
    filenames: &[&str],
) -> Option<std::path::PathBuf> {
    filenames
        .iter()
        .map(|filename| dir.join(filename))
        .find(|path| path.exists())
}

/// Try to find a model path, searching for the preferred model type first.
pub fn find_model_path(
    explicit_path: Option<&Path>,
    model_type: ModelType,
) -> Option<std::path::PathBuf> {
    // 1. Explicit path
    if let Some(p) = explicit_path {
        if p.exists() {
            return Some(p.to_path_buf());
        }
    }

    // 2. Environment variable
    if let Ok(env_path) = std::env::var("RAZEMIFY_MODEL_PATH") {
        let p = std::path::PathBuf::from(&env_path);
        if p.exists() {
            return Some(p);
        }
    }

    // 3. xtask cache directory (~/.razemify/models/)
    let filenames = model_filenames(model_type);

    if let Some(home) = dirs_path() {
        let cache_dir = home.join(".razemify").join("models");
        if let Some(path) = check_directory_for_models(&cache_dir, &filenames) {
            return Some(path);
        }
    }

    // 4. Legacy cache directory (~/.u2net/)
    if let Some(home) = dirs_path() {
        let cache_dir = home.join(".u2net");
        if let Some(path) = check_directory_for_models(&cache_dir, &filenames) {
            return Some(path);
        }
    }

    // 5. Current directory
    if let Some(path) = check_directory_for_models(&std::path::PathBuf::from("."), &filenames) {
        return Some(path);
    }

    // 6. Fallback: try any known model file in xtask cache
    let all_filenames = [
        "BiRefNet-general-bb_swin_v1_tiny-epoch_232.onnx",
        "BiRefNet-general-epoch_244.onnx",
        "u2net.onnx",
        "isnet-general-use.onnx",
    ];

    if let Some(home) = dirs_path() {
        let cache_dir = home.join(".razemify").join("models");
        if let Some(path) = check_directory_for_models(&cache_dir, &all_filenames) {
            return Some(path);
        }
    }

    // 7. Fallback: try any known model file in legacy cache
    if let Some(home) = dirs_path() {
        let cache_dir = home.join(".u2net");
        if let Some(path) = check_directory_for_models(&cache_dir, &all_filenames) {
            return Some(path);
        }
    }

    // 8. Fallback: try any known model file in current directory
    for filename in &all_filenames {
        let local = std::path::PathBuf::from(filename);
        if local.exists() {
            return Some(local);
        }
    }

    None
}

/// Return expected filenames for a given model type.
fn model_filenames(model_type: ModelType) -> Vec<&'static str> {
    match model_type {
        ModelType::BiRefNet => vec![
            "BiRefNet-general-bb_swin_v1_tiny-epoch_232.onnx",
            "BiRefNet-general-epoch_244.onnx",
        ],
        ModelType::U2Net => vec!["u2net.onnx"],
        ModelType::ISNet => vec!["isnet-general-use.onnx"],
    }
}

fn dirs_path() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("USERPROFILE")
            .ok()
            .map(std::path::PathBuf::from)
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("HOME").ok().map(std::path::PathBuf::from)
    }
}

/// Extract alpha channel from an existing RGBA image.
/// Used as fallback when no model is available.
pub fn extract_existing_alpha(img: &DynamicImage) -> Option<Vec<u8>> {
    if let DynamicImage::ImageRgba8(rgba) = img {
        let (w, h) = rgba.dimensions();
        let mut alpha = Vec::with_capacity((w * h) as usize);
        for y in 0..h {
            for x in 0..w {
                alpha.push(rgba.get_pixel(x, y)[3]);
            }
        }
        Some(alpha)
    } else {
        None
    }
}
