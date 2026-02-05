/// High-performance image posterization for Flutter using Rust.
///
/// Transform photos into artistic posterized images with multiple styles and color palettes.
library flutter_zanbergify;

export 'src/models.dart' show Preset, Palette, ProcessResult, ZanbergifyException;
export 'src/zanbergify.dart' show Zanbergify;
