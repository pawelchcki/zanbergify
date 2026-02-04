# WebGPU Diagnostics and Troubleshooting

## How to Check Which Backend is Being Used

### 1. Visual Indicator in UI
After the model loads, the status will show:
- `✓ BiRefNet ready (1024x1024, WebGPU 🚀)` - Using WebGPU (fast!)
- `✓ BiRefNet ready (1024x1024, WASM ⚡)` - Using WASM (slower)

### 2. Browser Console Logs
Open DevTools (F12) → Console tab and look for:

```
ONNX Runtime version: 1.20.0
Available backends: {...}
WebGPU API available in browser
WebGPU adapter found: {...}
Creating ONNX session with execution providers...
Attempting to create session with WebGPU...
✓ Session created with WebGPU backend
Model loaded successfully using WebGPU backend
```

### 3. Timing Information
When processing an image, check the console:

**WebGPU (Fast)**:
```
⚡ Inference took: 2500ms  (2.5 seconds)
✓ Total background removal: 4500ms (4.5s)
```

**WASM (Slow)**:
```
⚡ Inference took: 12000ms  (12 seconds)
✓ Total background removal: 15000ms (15s)
```

## WebGPU Requirements

### Browser Support
| Browser | Version | WebGPU Support |
|---------|---------|----------------|
| Chrome | 113+ | ✅ Full support |
| Edge | 113+ | ✅ Full support |
| Firefox | 133+ | ⚠️ Experimental (behind flag) |
| Safari | 18+ | ⚠️ Experimental |

### How to Check Your Browser

1. Open: chrome://gpu/ (or edge://gpu/)
2. Look for "WebGPU" section
3. Should say "WebGPU: Hardware accelerated"

### Enable WebGPU in Firefox (if needed)

1. Type `about:config` in address bar
2. Search for `dom.webgpu.enabled`
3. Set to `true`
4. Restart browser

## Common Issues

### Issue 1: "WebGPU API not available"

**Cause**: Browser doesn't support WebGPU
**Solution**:
- Update to Chrome 113+ or Edge 113+
- Check chrome://gpu/ to verify WebGPU is enabled

### Issue 2: "WebGPU adapter request failed"

**Cause**: Hardware/driver doesn't support WebGPU
**Solution**:
- Update GPU drivers
- Check if GPU supports Vulkan or DirectX 12
- Try Chrome/Edge instead of Firefox

### Issue 3: "WebGPU session creation failed"

**Cause**: ONNX Runtime can't create session with WebGPU
**Solution**:
- Check console for exact error message
- May need to use WASM backend (automatic fallback)
- Update ONNX Runtime version (change CDN URL in HTML)

### Issue 4: Still slow even with "WebGPU" shown

**Cause**: May be using WASM despite UI showing WebGPU
**Solution**:
- Check actual inference time in console
- Look for "Session created with WebGPU backend" message
- Verify chrome://gpu/ shows WebGPU enabled

## Testing WebGPU

### Quick Test in Console

Open DevTools → Console and run:

```javascript
// Check if WebGPU is available
console.log('WebGPU available:', !!navigator.gpu);

// Try to get adapter
navigator.gpu.requestAdapter().then(adapter => {
    if (adapter) {
        console.log('WebGPU adapter:', adapter);
        console.log('Features:', Array.from(adapter.features));
        console.log('Limits:', adapter.limits);
    } else {
        console.log('No WebGPU adapter available');
    }
}).catch(err => {
    console.error('WebGPU error:', err);
});
```

Expected output (if working):
```
WebGPU available: true
WebGPU adapter: GPUAdapter {...}
Features: ["texture-compression-bc", ...]
Limits: {maxTextureDimension2D: 16384, ...}
```

## Performance Benchmarks

### Expected Times (BiRefNet 1024x1024)

**With WebGPU**:
- Preprocessing: 200-500ms
- Inference: 2000-5000ms ⚡
- Postprocessing: 100-300ms
- Apply mask: 50-150ms
- **Total: 2500-6000ms (2.5-6s)**

**With WASM**:
- Preprocessing: 200-500ms
- Inference: 8000-15000ms 🐌
- Postprocessing: 100-300ms
- Apply mask: 50-150ms
- **Total: 8500-16000ms (8.5-16s)**

## Forcing WASM Backend (for testing)

If you want to test WASM backend explicitly, modify the code:

```javascript
// In loadBundledModel() function, change:
const sessionOptions = {
    executionProviders: ['wasm'],  // Force WASM
    graphOptimizationLevel: 'basic'
};
```

## Advanced Debugging

### Enable ONNX Runtime Logging

Add to JavaScript before creating session:

```javascript
ort.env.logLevel = 'verbose';
ort.env.debug = true;
```

### Check Execution Provider Actually Used

After creating session:

```javascript
console.log('Session executionProviders:', onnxSession.executionProviders);
```

### Monitor GPU Usage

**Windows**: Task Manager → Performance → GPU
**Linux**: `nvidia-smi` (for NVIDIA) or `radeontop` (for AMD)
**Mac**: Activity Monitor → GPU History

When inference runs, GPU usage should spike if WebGPU is working.

## Recommended Setup

### Best Performance
1. Chrome 113+ or Edge 113+
2. Dedicated GPU (NVIDIA, AMD, or Intel)
3. Updated GPU drivers
4. Hardware acceleration enabled in browser settings

### Fallback (Slower but Works)
1. Any modern browser
2. WASM backend (automatic fallback)
3. Integrated graphics OK
4. ~3x slower but reliable

## Reporting Issues

If WebGPU still doesn't work, collect this info:

1. Browser version: chrome://version/
2. GPU info: chrome://gpu/
3. Console logs from loading model
4. Inference timing from console
5. Operating system and GPU model

Then check:
- ONNX Runtime Web issues: https://github.com/microsoft/onnxruntime/issues
- WebGPU compatibility: https://caniuse.com/webgpu
