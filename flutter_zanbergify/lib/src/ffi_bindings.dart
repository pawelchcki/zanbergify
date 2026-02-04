import 'dart:ffi' as ffi;
import 'dart:io';
import 'dart:typed_data';
import 'package:ffi/ffi.dart';

/// FFI function signatures matching Rust implementation
typedef ProcessBytesNative = ffi.Int32 Function(
  ffi.Pointer<ffi.Uint8> inputData,
  ffi.Size inputLen,
  ffi.Pointer<ffi.Uint8> outputData,
  ffi.Pointer<ffi.Uint32> outputWidth,
  ffi.Pointer<ffi.Uint32> outputHeight,
  ffi.Pointer<Utf8> presetName,
  ffi.Pointer<Utf8> paletteName,
);

typedef ProcessBytesDart = int Function(
  ffi.Pointer<ffi.Uint8> inputData,
  int inputLen,
  ffi.Pointer<ffi.Uint8> outputData,
  ffi.Pointer<ffi.Uint32> outputWidth,
  ffi.Pointer<ffi.Uint32> outputHeight,
  ffi.Pointer<Utf8> presetName,
  ffi.Pointer<Utf8> paletteName,
);

typedef GetOutputSizeNative = ffi.Int32 Function(
  ffi.Pointer<ffi.Uint8> inputData,
  ffi.Size inputLen,
  ffi.Pointer<ffi.Uint32> widthOut,
  ffi.Pointer<ffi.Uint32> heightOut,
);

typedef GetOutputSizeDart = int Function(
  ffi.Pointer<ffi.Uint8> inputData,
  int inputLen,
  ffi.Pointer<ffi.Uint32> widthOut,
  ffi.Pointer<ffi.Uint32> heightOut,
);

typedef FreeStringNative = ffi.Void Function(ffi.Pointer<ffi.Uint8>);
typedef FreeStringDart = void Function(ffi.Pointer<ffi.Uint8>);

/// Load the native library
ffi.DynamicLibrary _loadLibrary() {
  if (Platform.isAndroid) {
    return ffi.DynamicLibrary.open('libflutter_zanbergify.so');
  } else if (Platform.isIOS || Platform.isMacOS) {
    return ffi.DynamicLibrary.process();
  } else if (Platform.isLinux) {
    return ffi.DynamicLibrary.open('libflutter_zanbergify.so');
  } else {
    throw UnsupportedError('Platform not supported');
  }
}

/// FFI bindings to the native Zanbergify library
class ZanbergifyBindings {
  static final ffi.DynamicLibrary _lib = _loadLibrary();

  static final ProcessBytesDart processBytes = _lib
      .lookup<ffi.NativeFunction<ProcessBytesNative>>('zanbergify_process_bytes')
      .asFunction();

  static final GetOutputSizeDart getOutputSize = _lib
      .lookup<ffi.NativeFunction<GetOutputSizeNative>>('zanbergify_get_output_size')
      .asFunction();

  static final FreeStringDart freeString = _lib
      .lookup<ffi.NativeFunction<FreeStringNative>>('zanbergify_free_string')
      .asFunction();
}
