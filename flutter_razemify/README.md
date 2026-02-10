# flutter_razemify

High-performance image posterization for Flutter using Rust. Transform photos into artistic posterized images with multiple styles and color palettes.

## Features

- **6 Built-in Presets**: 3 detailed algorithms + 3 comic-style algorithms
- **6 Color Palettes**: Original, Burgundy, Burgundy Teal, Burgundy Gold, Rose, and CMYK
- **High Performance**: Powered by Rust for fast native processing
- **Cross-Platform**: Android, iOS, Linux, and macOS support
- **No Prebuilt Binaries**: Rust code is compiled during build (requires Rust toolchain)

## Installation

Add to your `pubspec.yaml`:

```yaml
dependencies:
  flutter_razemify: ^0.1.0
```

## Requirements

- Rust toolchain (for building): [Install Rust](https://rustup.rs/)
- Flutter 3.3.0 or higher
- Dart SDK 3.6.0 or higher

### Platform-Specific Requirements

| Platform | Requirements |
|----------|-------------|
| Android  | NDK 25+ |
| iOS      | Xcode with Command Line Tools, CocoaPods |
| Linux    | CMake 3.10+, GCC/Clang |
| macOS    | Xcode with Command Line Tools, CocoaPods |

## Usage

```dart
import 'package:flutter_razemify/flutter_razemify.dart';
import 'dart:io';

// Simple usage - returns PNG bytes
final imageBytes = await File('input.jpg').readAsBytes();
final result = await Razemify.processImage(
  imageBytes,
  preset: Preset.comicBold,
  palette: Palette.burgundy,
);
await File('output.png').writeAsBytes(result);

// Advanced usage - get processing details
final detailedResult = await Razemify.processImageWithDetails(
  imageBytes,
  preset: Preset.detailedFine,
  palette: Palette.cmyk,
);
print('Processed in ${detailedResult.processingTime.inMilliseconds}ms');
final pngBytes = await detailedResult.toPng();
final jpegBytes = await detailedResult.toJpeg(quality: 95);
```

## Available Presets

- `Preset.detailedStandard` - Balanced detail and contrast
- `Preset.detailedStrong` - High contrast details
- `Preset.detailedFine` - Fine detail preservation
- `Preset.comicBold` - Bold comic-style edges
- `Preset.comicFine` - Fine comic-style edges
- `Preset.comicHeavy` - Heavy comic-style edges

## Available Palettes

- `Palette.original` - Red, white, black
- `Palette.burgundy` - Burgundy-based
- `Palette.burgundyTeal` - Burgundy and teal
- `Palette.burgundyGold` - Burgundy and gold
- `Palette.rose` - Rose-based
- `Palette.cmyk` - CMYK-inspired

## Example

See the [example app](example/lib/main.dart) for a full interactive demo with image picking and real-time preset/palette switching.

## Performance

Processing times on a modern device (example: Google Pixel 6):
- 1000x1000px image: ~100-200ms
- 2000x2000px image: ~300-500ms
- 4000x3000px image: ~800-1200ms

## Troubleshooting

### Rust Not Found

If you get a "Rustup not found" error during build:
1. Install Rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
2. Restart your terminal
3. Run `flutter clean` and rebuild

### Android NDK Issues

If Android build fails with NDK errors:
1. Open Android Studio
2. Go to Tools → SDK Manager → SDK Tools
3. Install NDK (version 25+)
4. Run `flutter clean` and rebuild

## License

This project is licensed under the MIT License - see the LICENSE file for details.

