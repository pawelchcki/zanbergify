# Model Loading

Models are loaded from external sources instead of being bundled with the deployment.

## Current Model

**BiRefNet-general-bb_swin_v1_tiny-epoch_232.onnx**

- **Type**: BiRefNet
- **Size**: 224 MB (too large for Cloudflare Pages 25 MB limit)
- **Input**: 1024x1024
- **Description**: High quality background removal with detailed edges
- **Loaded from**: https://github.com/danielgatis/rembg/releases/download/v0.0.0/BiRefNet-general-bb_swin_v1_tiny-epoch_232.onnx
- **SHA-256**: 5600024376f572a557870a5eb0afb1e5961636bef4e1e22132025467d0f03333

The model is downloaded directly from GitHub releases and cached in IndexedDB for subsequent loads.
