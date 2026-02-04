# ONNX Background Removal in Browser

This document describes the optional ONNX-based background removal feature added to the Zanbergify browser app.

## Overview

The browser app now supports optional background removal using ONNX Runtime Web. This feature uses progressive enhancement:

- **Core app**: Pure WASM posterization (~2.5MB) - always available
- **Background removal**: ONNX Runtime (~2-3MB) + models (5-200MB) - loaded on-demand

## Architecture

### Processing Pipeline

```
Image File → [Optional: ONNX background removal in JS] → WASM posterization → PNG output
```

1. **User uploads image**: Original image loaded into browser
2. **Optional background removal**: If enabled, ONNX model removes background and creates RGBA image
3. **Posterization**: WASM processor applies comic-style posterization (preserves alpha)
4. **Output**: Final PNG with transparent background (if enabled)

### Key Design Decisions

- **Pure JavaScript ONNX**: Keep WASM bundle small by handling ONNX in JS
- **Lazy loading**: ONNX Runtime only loads when user enables background removal
- **IndexedDB caching**: Models cached after first load (avoid re-downloading 5-200MB)
- **WebGPU first**: Prefer WebGPU backend for performance, fallback to WASM
- **Hybrid loading**: User upload (primary) + CDN URLs (convenience)

## Supported Models

### U2Net
- **Input size**: 320x320
- **Normalization**: Simple /255
- **Output processing**: Sigmoid activation → threshold at 0.5
- **File size**: ~4.7 MB (lite version)
- **Performance**: 0.5-2s (WebGPU), 2-5s (WASM backend)
- **Use case**: Fast, good quality for most images

### BiRefNet
- **Input size**: 1024x1024
- **Normalization**: ImageNet (mean=[0.485, 0.456, 0.406], std=[0.229, 0.224, 0.225])
- **Output processing**: Sigmoid → min-max normalization → threshold
- **File size**: ~200 MB (lite version ~50MB)
- **Performance**: 2-5s (WebGPU), 8-15s (WASM backend)
- **Use case**: High quality, detailed edges, complex scenes
- **Note**: Requires GraphOptimizationLevel::Basic due to ONNX Runtime bug

### ISNet
- **Input size**: 1024x1024
- **Normalization**: ImageNet (mean=[0.485, 0.456, 0.406], std=[0.229, 0.224, 0.225])
- **Output processing**: Sigmoid activation → threshold at 0.5
- **File size**: ~43 MB
- **Performance**: 1-3s (WebGPU), 5-10s (WASM backend)
- **Use case**: Balanced quality and speed

## Implementation Details

### Image Preprocessing

1. **Resize**: Scale to model input size (320x320 or 1024x1024) using Lanczos3
2. **Convert to CHW layout**: Transform HWC (Height, Width, Channels) to CHW (Channels, Height, Width)
3. **Normalize**:
   - U2Net: `pixel / 255.0`
   - BiRefNet/ISNet: `(pixel / 255.0 - mean) / std`
4. **Create tensor**: Float32Array with shape `[1, 3, H, W]`

### Inference

1. **Create session**: Load ONNX model with WebGPU or WASM execution provider
2. **Run inference**: Pass preprocessed tensor through model
3. **Extract output**: Get mask tensor with shape `[1, 1, H, W]`

### Mask Post-processing

1. **Apply activation**:
   - U2Net/ISNet: `sigmoid(x) = 1.0 / (1.0 + exp(-x))`
   - BiRefNet: `sigmoid(x)` → min-max normalize to [0, 1]
2. **Threshold**: Binary mask at 0.5 (128 in 0-255 range)
3. **Resize**: Scale mask back to original image dimensions
4. **Apply to image**: Set alpha channel based on mask

### Integration with WASM

The WASM processor already handles RGBA images correctly via `extract_alpha_from_image()`:

```javascript
// JavaScript: Create RGBA with mask as alpha
const rgbaBytes = await applyMaskToImage(imageBytes, mask, width, height);

// Pass to WASM - alpha is preserved automatically
const resultBytes = ZanbergifyProcessor.processImage(rgbaBytes, params, palette);
```

## Usage

### Enable Background Removal

1. Check "Enable Background Removal" checkbox
2. Controls section appears

### Load Model

#### Option 1: Upload Model File
1. Select "Upload Model File" from Model Source
2. Click "Choose File" and select .onnx file
3. Model type auto-detected from filename (or choose manually)
4. Model loads and is cached in IndexedDB

#### Option 2: CDN Model
1. Select "CDN: U2Net Lite" from Model Source
2. Model automatically downloads and caches
3. ~4.7 MB download (one-time)

#### Option 3: Custom URL
1. Select "Custom URL" from Model Source
2. Enter full URL to .onnx model
3. Click outside input to trigger load
4. Model downloads and caches

### Process Images

Once model is loaded:
1. Upload image as normal
2. Background removal happens automatically before posterization
3. Final image has transparent background + posterization effect

## Performance

### Model Loading
- **First time (CDN)**: 3-10 seconds depending on model size and connection
- **Cached (IndexedDB)**: < 1 second

### Inference Time

| Model | Resolution | WebGPU | WASM Backend |
|-------|-----------|--------|--------------|
| U2Net | 320x320 | 0.5-2s | 2-5s |
| BiRefNet | 1024x1024 | 2-5s | 8-15s |
| ISNet | 1024x1024 | 1-3s | 5-10s |

### Bundle Size
- **Initial load**: ~2.5 MB (no change - ONNX not included)
- **ONNX Runtime**: ~2-3 MB (loaded dynamically when enabled)
- **Models**: 5-200 MB (user-provided or CDN, cached separately)

## Browser Compatibility

### WebGPU Support (Best Performance)
- Chrome/Edge 113+: Full support
- Safari 18+: Experimental support
- Firefox: Not yet supported (uses WASM backend)

### Minimum Requirements
- Chrome/Edge 90+
- Firefox 90+
- Safari 15+
- IndexedDB support (all modern browsers)

## Caching Strategy

### IndexedDB Cache
- **Database**: `zanbergify_models`
- **Object Store**: `models`
- **Key format**:
  - File uploads: `file_{filename}_{size}`
  - URL downloads: `url_{url}`
- **Storage**: Unlimited (browser-dependent, typically 50-100MB+)

### Cache Management
Models persist across sessions until:
- User clears browser data
- IndexedDB quota exceeded (automatic cleanup)
- Manual cache clearing (future feature)

## Where to Get Models

### Recommended Sources

1. **Hugging Face**: [https://huggingface.co/models?library=onnx](https://huggingface.co/models?library=onnx)
   - Search for "u2net", "birefnet", "isnet"
   - Download .onnx files directly

2. **Pre-trained Models**:
   - U2Net: [rembg models](https://github.com/danielgatis/rembg#models)
   - BiRefNet: [ZhengPeng7/BiRefNet](https://github.com/ZhengPeng7/BiRefNet)
   - ISNet: [xuebinqin/DIS](https://github.com/xuebinqin/DIS)

3. **Convert PyTorch Models**:
   ```bash
   # Install ONNX tools
   pip install torch onnx onnxruntime

   # Convert model
   python convert_to_onnx.py --model birefnet --output model.onnx
   ```

### CDN Models

The app includes pre-configured CDN links for convenience:
- **U2Net Lite**: Hosted on Hugging Face CDN (~4.7 MB)

Note: CDN availability may vary. Always verify model URLs are accessible.

## Troubleshooting

### Model Won't Load
- **Check file format**: Must be .onnx file
- **Check file size**: Large models (>200MB) may exceed browser limits
- **Check browser console**: Look for specific error messages
- **Try different model**: Start with U2Net Lite (smallest)

### Slow Performance
- **Check execution provider**: WebGPU is 3-5x faster than WASM
- **Update browser**: Chrome/Edge 113+ for WebGPU support
- **Try smaller model**: U2Net is fastest
- **Disable auto-process**: Manually trigger processing to avoid repeated inference

### Background Not Removed Properly
- **Check model type**: Ensure correct type selected (auto-detect or manual)
- **Try different model**: Some models work better on certain image types
- **Check input image**: High contrast images work best
- **Adjust threshold**: Future feature - currently fixed at 0.5

### IndexedDB Errors
- **Check storage quota**: Browser may have reached storage limit
- **Clear browser data**: Reset IndexedDB and try again
- **Check private mode**: Some browsers restrict IndexedDB in private/incognito mode

## Technical Reference

### File Locations
- **HTML**: `/www/index.html` - UI controls
- **JavaScript**: `/www/index.js` - ONNX integration
- **Reference implementation**: `/rust/zanbergify-core/src/rembg.rs`

### Key Functions

#### `initOnnxRuntime()`
Dynamically imports ONNX Runtime Web from CDN.

#### `loadModel(source, sourceType)`
Loads model from file or URL, handles caching.

#### `preprocessImage(imageData, modelType)`
Resizes and normalizes image for model input.

#### `runOnnxInference(inputTensor)`
Executes model inference.

#### `postprocessMask(outputTensor, modelType, origWidth, origHeight)`
Applies activation, threshold, and resize to mask.

#### `applyMaskToImage(imageBytes, mask, width, height)`
Creates RGBA image with mask as alpha channel.

#### `processImageWithRembg(imageBytes)`
Main pipeline for background removal.

### Constants

```javascript
// ImageNet normalization
const IMAGENET_MEAN = [0.485, 0.456, 0.406];
const IMAGENET_STD = [0.229, 0.224, 0.225];

// Model input sizes
const MODEL_INPUT_SIZE = {
    u2net: 320,
    birefnet: 1024,
    isnet: 1024
};

// Threshold for binary mask
const MASK_THRESHOLD = 128; // 0.5 in 0-255 range
```

## Future Enhancements

### Planned Features
- [ ] Model size warnings before download
- [ ] Cache management UI (view/clear cached models)
- [ ] Web Worker support for off-main-thread inference
- [ ] Adjustable mask threshold slider
- [ ] Three-panel preview (original → mask → final)
- [ ] Mask overlay toggle
- [ ] Download mask separately
- [ ] Resume capability for interrupted downloads
- [ ] Multiple model comparison

### Performance Optimizations
- [ ] Profile preprocessing overhead
- [ ] Optimize mask resize algorithm
- [ ] Reduce memory allocations
- [ ] Implement model quantization support
- [ ] Add LRU cache for multiple models

## License

Same license as parent project (Zanbergify).

## Credits

- **ONNX Runtime**: [microsoft/onnxruntime](https://github.com/microsoft/onnxruntime)
- **U2Net**: [xuebinqin/U-2-Net](https://github.com/xuebinqin/U-2-Net)
- **BiRefNet**: [ZhengPeng7/BiRefNet](https://github.com/ZhengPeng7/BiRefNet)
- **ISNet**: [xuebinqin/DIS](https://github.com/xuebinqin/DIS)
