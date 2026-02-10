pub mod clahe;
pub mod comic_pipeline;
pub mod edge_detect;
pub mod exif_orientation;

#[cfg(all(not(target_arch = "wasm32"), feature = "rembg"))]
pub mod ffi;

#[cfg(feature = "flutter_ffi")]
pub mod flutter_ffi;

pub mod pipeline;
pub mod posterize;

#[cfg(feature = "rembg")]
pub mod rembg;

pub mod sharpen;
