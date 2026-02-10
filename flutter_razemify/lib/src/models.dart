import 'dart:typed_data';
import 'package:image/image.dart' as img;

/// Posterization presets available for image processing.
enum Preset {
  /// Balanced detail and contrast
  detailedStandard('detailed_standard'),

  /// High contrast details
  detailedStrong('detailed_strong'),

  /// Fine detail preservation
  detailedFine('detailed_fine'),

  /// Bold comic-style edges
  comicBold('comic_bold'),

  /// Fine comic-style edges
  comicFine('comic_fine'),

  /// Heavy comic-style edges
  comicHeavy('comic_heavy');

  const Preset(this.value);
  final String value;
}

/// Color palettes available for image processing.
enum Palette {
  /// Red, white, black
  original('original'),

  /// Burgundy-based
  burgundy('burgundy'),

  /// Burgundy and teal
  burgundyTeal('burgundy_teal'),

  /// Burgundy and gold
  burgundyGold('burgundy_gold'),

  /// Rose-based
  rose('rose'),

  /// CMYK-inspired
  cmyk('cmyk');

  const Palette(this.value);
  final String value;
}

/// Result of image processing with RGB data and metadata.
class ProcessResult {
  /// RGB pixel data (width * height * 3 bytes)
  final Uint8List rgbData;

  /// Image width in pixels
  final int width;

  /// Image height in pixels
  final int height;

  /// Time taken to process the image
  final Duration processingTime;

  const ProcessResult({
    required this.rgbData,
    required this.width,
    required this.height,
    required this.processingTime,
  });

  /// Convert RGB data to PNG bytes.
  Future<Uint8List> toPng() async {
    final image = img.Image.fromBytes(
      width: width,
      height: height,
      bytes: rgbData.buffer,
      numChannels: 3,
    );
    return Uint8List.fromList(img.encodePng(image));
  }

  /// Convert RGB data to JPEG bytes with specified quality (0-100).
  Future<Uint8List> toJpeg({int quality = 90}) async {
    final image = img.Image.fromBytes(
      width: width,
      height: height,
      bytes: rgbData.buffer,
      numChannels: 3,
    );
    return Uint8List.fromList(img.encodeJpg(image, quality: quality));
  }
}

/// Exception thrown when image processing fails.
class RazemifyException implements Exception {
  final String message;
  final int? errorCode;

  const RazemifyException(this.message, [this.errorCode]);

  @override
  String toString() {
    if (errorCode != null) {
      return 'RazemifyException($errorCode): $message';
    }
    return 'RazemifyException: $message';
  }
}
