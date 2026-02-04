# Bundled ONNX Models

This directory contains pre-downloaded ONNX models for background removal.

## BiRefNet-general-bb_swin_v1_tiny-epoch_232.onnx

- **Size**: 214 MB
- **Type**: BiRefNet (auto-detected)
- **Input Resolution**: 1024x1024
- **Normalization**: ImageNet (mean=[0.485, 0.456, 0.406], std=[0.229, 0.224, 0.225])
- **Output Processing**: Sigmoid + min-max normalization
- **Expected Performance**:
  - WebGPU: 2-5 seconds
  - WASM backend: 8-15 seconds
- **Quality**: Excellent - Best for detailed edges and complex scenes
- **Source**: rembg v0.0.0 release
- **URL**: https://github.com/danielgatis/rembg/releases/download/v0.0.0/BiRefNet-general-bb_swin_v1_tiny-epoch_232.onnx

## Usage

Select "Bundled: BiRefNet Lite" from the Model Source dropdown in the app.
The model will load from this local directory (no download needed).

## Model Caching

The model is cached in IndexedDB after first load, so subsequent page loads
will be instant (< 1 second).

## License

Check the source repository for model license information:
- BRIA AI RMBG-1.4: https://huggingface.co/briaai/RMBG-1.4

## Adding More Models

To bundle additional models:

1. Download the .onnx file
2. Place it in this directory
3. Update `www/index.html` to add a dropdown option
4. Update `www/index.js` to handle the new option

Example:
```javascript
if (source === 'bundled-mymodel') {
    modelTypeSelect.value = 'u2net'; // or birefnet, isnet
    loadModel('models/mymodel.onnx', 'url')
        .catch(error => {
            showStatus(`Failed to load bundled model: ${error.message}`, 'error');
        });
}
```
