import 'dart:ffi' as ffi;
import 'dart:isolate';
import 'dart:typed_data';
import 'package:ffi/ffi.dart';
import 'package:image/image.dart' as img;

import 'ffi_bindings.dart';
import 'models.dart';

/// Main API for Zanbergify image processing.
class Zanbergify {
  /// Process an image and return PNG bytes.
  ///
  /// This is a simple convenience method that returns PNG-encoded bytes.
  /// For more control over the output format and access to processing metadata,
  /// use [processImageWithDetails].
  ///
  /// Throws [ZanbergifyException] if processing fails.
  static Future<Uint8List> processImage(
    Uint8List imageBytes, {
    required Preset preset,
    required Palette palette,
  }) async {
    final result = await processImageWithDetails(
      imageBytes,
      preset: preset,
      palette: palette,
    );
    return result.toPng();
  }

  /// Process an image and return detailed results.
  ///
  /// Returns a [ProcessResult] containing RGB data, dimensions, and timing information.
  /// You can convert the result to PNG or JPEG using [ProcessResult.toPng] or
  /// [ProcessResult.toJpeg].
  ///
  /// Processing runs on a separate isolate to avoid blocking the UI.
  ///
  /// Throws [ZanbergifyException] if processing fails.
  static Future<ProcessResult> processImageWithDetails(
    Uint8List imageBytes, {
    required Preset preset,
    required Palette palette,
  }) async {
    final params = _ProcessParams(
      imageBytes: imageBytes,
      preset: preset.value,
      palette: palette.value,
    );

    return Isolate.run(() => _processInIsolate(params));
  }

  /// Internal: Process image in isolate
  static ProcessResult _processInIsolate(_ProcessParams params) {
    final stopwatch = Stopwatch()..start();

    // Step 1: Allocate input buffer
    final inputPtr = malloc.allocate<ffi.Uint8>(params.imageBytes.length);
    final inputList = inputPtr.asTypedList(params.imageBytes.length);
    inputList.setAll(0, params.imageBytes);

    // Step 2: Get output dimensions FIRST
    final widthPtr = malloc.allocate<ffi.Uint32>(ffi.sizeOf<ffi.Uint32>());
    final heightPtr = malloc.allocate<ffi.Uint32>(ffi.sizeOf<ffi.Uint32>());

    int errorCode = ZanbergifyBindings.getOutputSize(
      inputPtr,
      params.imageBytes.length,
      widthPtr,
      heightPtr,
    );

    if (errorCode != 0) {
      malloc.free(inputPtr);
      malloc.free(widthPtr);
      malloc.free(heightPtr);
      throw ZanbergifyException(
        'Failed to get output size: ${_getErrorMessage(errorCode)}',
        errorCode,
      );
    }

    final width = widthPtr.value;
    final height = heightPtr.value;
    final outputSize = width * height * 3; // RGB: 3 bytes per pixel

    // Step 3: Allocate output buffer
    final outputPtr = malloc.allocate<ffi.Uint8>(outputSize);

    // Step 4: Convert preset/palette strings to C strings
    final presetPtr = params.preset.toNativeUtf8();
    final palettePtr = params.palette.toNativeUtf8();

    try {
      // Step 5: Call processing function with pre-allocated output buffer
      errorCode = ZanbergifyBindings.processBytes(
        inputPtr,
        params.imageBytes.length,
        outputPtr,      // Pass pre-allocated output buffer
        widthPtr,       // Rust will write dimensions here
        heightPtr,
        presetPtr,
        palettePtr,
      );

      if (errorCode != 0) {
        throw ZanbergifyException(
          'Image processing failed: ${_getErrorMessage(errorCode)}',
          errorCode,
        );
      }

      // Step 6: Copy RGB data from native buffer to Dart
      final rgbData = Uint8List.fromList(
        outputPtr.asTypedList(outputSize),
      );

      stopwatch.stop();

      return ProcessResult(
        rgbData: rgbData,
        width: width,
        height: height,
        processingTime: stopwatch.elapsed,
      );
    } finally {
      // Step 7: Free ALL allocated memory
      malloc.free(inputPtr);
      malloc.free(outputPtr);
      malloc.free(widthPtr);
      malloc.free(heightPtr);
      malloc.free(presetPtr);
      malloc.free(palettePtr);
    }
  }

  /// Helper method to translate error codes to messages
  static String _getErrorMessage(int code) {
    switch (code) {
      case -1:
        return 'Invalid preset name encoding or failed to decode image';
      case -2:
        return 'Invalid palette name encoding';
      case -3:
        return 'Failed to decode image';
      case -4:
        return 'Unknown preset name';
      case -5:
        return 'Unknown palette name';
      case -6:
        return 'Processing failed';
      case 0:
        return 'Success';
      default:
        return 'Unknown error (code: $code)';
    }
  }
}

/// Internal class for passing parameters to isolate
class _ProcessParams {
  final Uint8List imageBytes;
  final String preset;
  final String palette;

  _ProcessParams({
    required this.imageBytes,
    required this.preset,
    required this.palette,
  });
}
