import init, { ZanbergifyProcessor, DetailedParams, ColorPalette } from '../pkg/zanbergify_wasm.js';
// Import ONNX Runtime Web latest version with WebGPU support
import * as ort from 'https://cdn.jsdelivr.net/npm/onnxruntime-web@1.23.2/dist/ort.webgpu.min.mjs';

// Make ort globally available for compatibility
window.ort = ort;

let wasmInitialized = false;
let currentImageBytes = null;

// ONNX Runtime state
let onnxSession = null;
let currentModelType = null;

// IndexedDB for model caching
const DB_NAME = 'zanbergify_models';
const DB_VERSION = 1;
const STORE_NAME = 'models';

// DOM elements
const imageInput = document.getElementById('imageInput');
const presetSelect = document.getElementById('presetSelect');
const paletteSelect = document.getElementById('paletteSelect');
const autoProcessCheckbox = document.getElementById('autoProcess');
const processBtn = document.getElementById('processBtn');
const statusDiv = document.getElementById('status');
const originalWrapper = document.getElementById('originalWrapper');
const resultWrapper = document.getElementById('resultWrapper');

// Background removal DOM elements
const enableRembgCheckbox = document.getElementById('enableRembg');
const rembgControls = document.getElementById('rembgControls');
const modelStatusDiv = document.getElementById('modelStatus');
const rembgProgressDiv = document.getElementById('rembgProgress');
const rembgProgressBar = document.getElementById('rembgProgressBar');
const rembgProgressText = document.getElementById('rembgProgressText');

// Slider elements
const threshLowSlider = document.getElementById('threshLow');
const threshLowValue = document.getElementById('threshLowValue');
const threshHighSlider = document.getElementById('threshHigh');
const threshHighValue = document.getElementById('threshHighValue');
const clipLimitSlider = document.getElementById('clipLimit');
const clipLimitValue = document.getElementById('clipLimitValue');
const tileSizeSlider = document.getElementById('tileSize');
const tileSizeValue = document.getElementById('tileSizeValue');

// Processing state
let processingTimeout = null;
let isProcessing = false;

// Initialize WASM module
async function initWasm() {
    try {
        showStatus('Initializing WASM module...', 'info');
        await init();
        wasmInitialized = true;
        showStatus('Ready to process images', 'success');
        setTimeout(() => {
            statusDiv.innerHTML = '';
        }, 2000);
    } catch (error) {
        showStatus(`Failed to initialize WASM: ${error.message}`, 'error');
        console.error('WASM initialization error:', error);
    }
}

// ========== ONNX Runtime Integration ==========

// Initialize IndexedDB for model caching
function openDB() {
    return new Promise((resolve, reject) => {
        const request = indexedDB.open(DB_NAME, DB_VERSION);

        request.onerror = () => reject(request.error);
        request.onsuccess = () => resolve(request.result);

        request.onupgradeneeded = (event) => {
            const db = event.target.result;
            if (!db.objectStoreNames.contains(STORE_NAME)) {
                db.createObjectStore(STORE_NAME);
            }
        };
    });
}

// Cache model in IndexedDB
async function cacheModel(key, data) {
    try {
        const db = await openDB();
        const tx = db.transaction(STORE_NAME, 'readwrite');
        const store = tx.objectStore(STORE_NAME);
        await new Promise((resolve, reject) => {
            const request = store.put(data, key);
            request.onsuccess = () => resolve();
            request.onerror = () => reject(request.error);
        });
        db.close();
    } catch (error) {
        console.error('Failed to cache model:', error);
    }
}

// Retrieve cached model from IndexedDB
async function getCachedModel(key) {
    try {
        const db = await openDB();
        const tx = db.transaction(STORE_NAME, 'readonly');
        const store = tx.objectStore(STORE_NAME);
        const result = await new Promise((resolve, reject) => {
            const request = store.get(key);
            request.onsuccess = () => resolve(request.result);
            request.onerror = () => reject(request.error);
        });
        db.close();
        return result;
    } catch (error) {
        console.error('Failed to retrieve cached model:', error);
        return null;
    }
}

// Initialize ONNX Runtime (check if loaded from script tag)
async function initOnnxRuntime() {
    try {
        // Check if ort is available globally
        if (typeof ort === 'undefined') {
            throw new Error('ONNX Runtime not loaded. Check if the script tag is present.');
        }

        // Verify InferenceSession exists
        if (!ort.InferenceSession) {
            throw new Error('ONNX Runtime loaded but InferenceSession not found');
        }

        console.log('ONNX Runtime version:', ort.version);

        // Configure WASM file paths for WebGPU support
        // Point to CDN where WASM files are hosted
        ort.env.wasm.wasmPaths = 'https://cdn.jsdelivr.net/npm/onnxruntime-web@1.23.2/dist/';

        // Enable WebGPU if available
        ort.env.wasm.numThreads = 4;
        ort.env.wasm.simd = true;

        console.log('ONNX Runtime environment:', {
            wasmPaths: ort.env.wasm.wasmPaths,
            numThreads: ort.env.wasm.numThreads,
            simd: ort.env.wasm.simd
        });

        // Check WebGPU availability
        if (navigator.gpu) {
            console.log('✓ WebGPU API available in browser');
            try {
                const adapter = await navigator.gpu.requestAdapter();
                if (adapter) {
                    console.log('✓ WebGPU adapter found:', adapter);
                    console.log('  Features:', Array.from(adapter.features));
                    console.log('  Limits (max texture):', adapter.limits.maxTextureDimension2D);
                } else {
                    console.warn('⚠️ WebGPU adapter request failed');
                }
            } catch (e) {
                console.warn('⚠️ WebGPU adapter error:', e);
            }
        } else {
            console.warn('⚠️ WebGPU API not available in this browser');
            console.log('  Using WASM backend (slower but compatible)');
        }

        return ort;
    } catch (error) {
        throw new Error(`Failed to initialize ONNX Runtime: ${error.message}`);
    }
}

// Detect model type from filename
function detectModelType(filename) {
    const lower = filename.toLowerCase();
    if (lower.includes('birefnet')) return 'birefnet';
    if (lower.includes('isnet')) return 'isnet';
    if (lower.includes('u2net')) return 'u2net';
    return null;
}

// Get model input size based on type
function getModelInputSize(modelType) {
    switch (modelType) {
        case 'u2net':
            return 320;
        case 'birefnet':
        case 'isnet':
        case 'rmbg':
            return 1024;
        default:
            return 320;
    }
}

// Load bundled model with fallback
async function loadBundledModel() {
    try {
        await initOnnxRuntime();

        // Try RMBG-1.4 first (state-of-the-art, 1024x1024)
        // Fall back to U2Net if it fails (faster, 320x320)
        const modelConfigs = [
            {
                type: 'rmbg',
                url: 'https://zanbergify-models-cdn.pawelchcki.workers.dev/rmbg-1.4.onnx',
                cacheKey: 'bundled_rmbg_1.4',
                description: 'RMBG-1.4 (1024x1024, state-of-the-art)',
                // Conservative settings for large high-resolution model
                sessionOptions: {
                    webgpu: {
                        executionProviders: [{
                            name: 'webgpu',
                            preferredLayout: 'NHWC',
                            deviceType: 'gpu',
                            powerPreference: 'high-performance'
                        }],
                        graphOptimizationLevel: 'disabled',    // Disable to save memory
                        executionMode: 'sequential',
                        enableMemPattern: false,               // Disable to avoid OOM
                        enableCpuMemArena: false,
                        logSeverityLevel: 0,
                        logVerbosityLevel: 0
                    },
                    wasm: {
                        executionProviders: ['wasm'],
                        graphOptimizationLevel: 'all',
                        enableCpuMemArena: true,
                        enableMemPattern: true
                    }
                }
            },
            {
                type: 'u2net',
                url: 'https://zanbergify-models-cdn.pawelchcki.workers.dev/u2net.onnx',
                cacheKey: 'bundled_u2net',
                description: 'U2Net (320x320, fast)',
                // More aggressive optimization for smaller model
                sessionOptions: {
                    webgpu: {
                        executionProviders: [{
                            name: 'webgpu',
                            preferredLayout: 'NHWC',
                            deviceType: 'gpu',
                            powerPreference: 'high-performance'
                        }],
                        graphOptimizationLevel: 'all',         // Enable optimizations
                        executionMode: 'parallel',             // Parallel for speed
                        enableMemPattern: true,                // Safe for smaller model
                        enableCpuMemArena: false,
                        logSeverityLevel: 0,
                        logVerbosityLevel: 0
                    },
                    wasm: {
                        executionProviders: ['wasm'],
                        graphOptimizationLevel: 'all',
                        enableCpuMemArena: true,
                        enableMemPattern: true
                    }
                }
            }
        ];

        let lastError = null;

        for (const config of modelConfigs) {
            try {
                console.log(`Attempting to load ${config.description}...`);
                await loadModelFromConfig(config);
                return; // Success!
            } catch (error) {
                console.warn(`Failed to load ${config.description}:`, error.message);
                lastError = error;
                // Continue to next model
            }
        }

        // If we get here, all models failed
        throw lastError || new Error('All models failed to load');
    } catch (error) {
        hideProgressDiv();
        modelStatusDiv.textContent = `Failed to load model: ${error.message}`;
        modelStatusDiv.style.background = '#ffebee';
        modelStatusDiv.style.color = '#c62828';
        throw error;
    }
}

// Load a specific model configuration
async function loadModelFromConfig(config) {
    currentModelType = config.type;
    const modelUrl = config.url;
    const cacheKey = config.cacheKey;

    let modelData = null;

    // Check cache first
    const cached = await getCachedModel(cacheKey);

    if (cached) {
        showProgressDiv(`Loading ${config.description} from cache...`, 10);
        modelData = cached;
    } else {
        // Load from bundled file
        showProgressDiv(`Loading ${config.description}...`, 10);
        const response = await fetch(modelUrl);
        if (!response.ok) {
            throw new Error(`Failed to load model: ${response.statusText}`);
        }

        const contentLength = response.headers.get('content-length');
        const total = contentLength ? parseInt(contentLength, 10) : 0;
        let loaded = 0;

        const reader = response.body.getReader();
        const chunks = [];

        while (true) {
            const { done, value } = await reader.read();
            if (done) break;

            chunks.push(value);
            loaded += value.length;

            if (total > 0) {
                const progress = Math.round((loaded / total) * 80) + 10;
                showProgressDiv(`Loading ${config.description}... ${Math.round(loaded / 1024 / 1024)}MB / ${Math.round(total / 1024 / 1024)}MB`, progress);
            }
        }

        const allChunks = new Uint8Array(loaded);
        let position = 0;
        for (const chunk of chunks) {
            allChunks.set(chunk, position);
            position += chunk.length;
        }

        modelData = allChunks;

        // Cache the model
        await cacheModel(cacheKey, modelData);
    }

    // Create ONNX session
    showProgressDiv(`Initializing ${config.description}...`, 90);

    console.log(`Creating ONNX session for ${config.description}...`);

    let usedBackend = 'wasm';  // Default to WASM
    let sessionOptions;
    let sessionCreated = false;

    // Detailed WebGPU detection
    console.log('=== WebGPU Detection ===');
    console.log('navigator.gpu:', navigator.gpu);

    if (navigator.gpu) {
        try {
            const adapter = await navigator.gpu.requestAdapter({
                powerPreference: 'high-performance'
            });
            console.log('GPU Adapter:', adapter);
            console.log('Adapter limits:', adapter.limits);

            if (adapter) {
                // Request device with higher limits for large models
                const device = await adapter.requestDevice({
                    requiredLimits: {
                        maxStorageBuffersPerShaderStage: Math.min(
                            adapter.limits.maxStorageBuffersPerShaderStage,
                            16
                        ),
                        maxStorageBufferBindingSize: adapter.limits.maxStorageBufferBindingSize,
                        maxBufferSize: adapter.limits.maxBufferSize,
                        maxComputeWorkgroupStorageSize: adapter.limits.maxComputeWorkgroupStorageSize
                    }
                });
                console.log('GPU Device:', device);
                console.log('Device limits:', device.limits);
                console.log('✓ GPU device with enhanced limits');
            }
        } catch (err) {
            console.error('WebGPU adapter/device request failed:', err);
        }
    }

    // Try WebGPU if available
    if (navigator.gpu) {
        try {
            // Use model-specific WebGPU configuration
            const webgpuConfig = config.sessionOptions.webgpu;

            console.log('Attempting WebGPU config...', webgpuConfig);
            onnxSession = await ort.InferenceSession.create(modelData, webgpuConfig);

            // Verify session was created successfully
            if (!onnxSession || !onnxSession.inputNames || onnxSession.inputNames.length === 0) {
                throw new Error('Session created but invalid (no input names)');
            }

            usedBackend = 'webgpu';
            sessionCreated = true;
            console.log('✓ Session created with WebGPU backend');
            console.log('  Input names:', onnxSession.inputNames);
            console.log('  Output names:', onnxSession.outputNames);
        } catch (err) {
            console.warn('WebGPU config failed:', err?.message || err || 'Unknown error');
            if (err?.stack) console.warn('  Stack:', err.stack);

            // Clean up failed session attempt
            if (onnxSession) {
                try {
                    await onnxSession.release?.();
                } catch (e) {
                    // Ignore cleanup errors
                }
                onnxSession = null;
            }
        }
    }

    // Fallback to WASM if WebGPU failed
    if (!sessionCreated) {
        console.log('Falling back to WASM backend...');
        // Use model-specific WASM configuration
        const wasmConfig = config.sessionOptions.wasm;

        try {
            onnxSession = await ort.InferenceSession.create(modelData, wasmConfig);
            usedBackend = 'wasm';
            console.log('✓ Session created with WASM backend');
        } catch (wasmError) {
            console.error('WASM session creation also failed:', wasmError);
            throw new Error(`Failed to create session: ${wasmError.message}`);
        }
    }

    // Log which execution provider is actually being used
    if (onnxSession) {
        console.log('Session input names:', onnxSession.inputNames);
        console.log('Session output names:', onnxSession.outputNames);
    }

    hideProgressDiv();

    const backendEmoji = usedBackend === 'webgpu' ? '🚀' : '⚡';
    const backendText = usedBackend === 'webgpu' ? 'WebGPU' : 'WASM';
    const inputSize = getModelInputSize(currentModelType);

    modelStatusDiv.textContent = `✓ ${config.description} ready (${inputSize}x${inputSize}, ${backendText} ${backendEmoji})`;
    modelStatusDiv.style.background = '#e8f5e9';
    modelStatusDiv.style.color = '#2e7d32';
    modelStatusDiv.style.cursor = 'default';

    console.log(`Model ${config.description} loaded successfully using ${backendText} backend`);

    // Trigger reprocessing if image is loaded
    scheduleAutoProcess();
}

// Show progress indicator
function showProgressDiv(text, progress) {
    rembgProgressDiv.style.display = 'block';
    rembgProgressBar.value = progress;
    rembgProgressText.textContent = text;
}

// Hide progress indicator
function hideProgressDiv() {
    rembgProgressDiv.style.display = 'none';
}

// Preprocess image for ONNX inference
async function preprocessImage(imageData, modelType) {
    const inputSize = getModelInputSize(modelType);

    // Create canvas for resizing
    const canvas = document.createElement('canvas');
    canvas.width = inputSize;
    canvas.height = inputSize;
    const ctx = canvas.getContext('2d');

    // Draw and resize image
    const img = await createImageBitmap(imageData);
    ctx.drawImage(img, 0, 0, inputSize, inputSize);

    // Get pixel data
    const imageDataResized = ctx.getImageData(0, 0, inputSize, inputSize);
    const pixels = imageDataResized.data;

    // Convert to CHW layout and normalize
    const tensorData = new Float32Array(3 * inputSize * inputSize);

    if (modelType === 'u2net') {
        // Simple /255 normalization
        for (let y = 0; y < inputSize; y++) {
            for (let x = 0; x < inputSize; x++) {
                const idx = (y * inputSize + x) * 4;
                tensorData[0 * inputSize * inputSize + y * inputSize + x] = pixels[idx + 0] / 255.0;
                tensorData[1 * inputSize * inputSize + y * inputSize + x] = pixels[idx + 1] / 255.0;
                tensorData[2 * inputSize * inputSize + y * inputSize + x] = pixels[idx + 2] / 255.0;
            }
        }
    } else if (modelType === 'rmbg') {
        // RMBG normalization: mean=[0.5, 0.5, 0.5], std=[1.0, 1.0, 1.0]
        const RMBG_MEAN = [0.5, 0.5, 0.5];
        const RMBG_STD = [1.0, 1.0, 1.0];

        for (let y = 0; y < inputSize; y++) {
            for (let x = 0; x < inputSize; x++) {
                const idx = (y * inputSize + x) * 4;
                for (let c = 0; c < 3; c++) {
                    const val = pixels[idx + c] / 255.0;
                    tensorData[c * inputSize * inputSize + y * inputSize + x] =
                        (val - RMBG_MEAN[c]) / RMBG_STD[c];
                }
            }
        }
    } else {
        // ImageNet normalization for BiRefNet and ISNet
        const IMAGENET_MEAN = [0.485, 0.456, 0.406];
        const IMAGENET_STD = [0.229, 0.224, 0.225];

        for (let y = 0; y < inputSize; y++) {
            for (let x = 0; x < inputSize; x++) {
                const idx = (y * inputSize + x) * 4;
                for (let c = 0; c < 3; c++) {
                    const val = pixels[idx + c] / 255.0;
                    tensorData[c * inputSize * inputSize + y * inputSize + x] =
                        (val - IMAGENET_MEAN[c]) / IMAGENET_STD[c];
                }
            }
        }
    }

    return new ort.Tensor('float32', tensorData, [1, 3, inputSize, inputSize]);
}

// Run ONNX inference
async function runOnnxInference(inputTensor) {
    const feeds = {};
    feeds[onnxSession.inputNames[0]] = inputTensor;
    const results = await onnxSession.run(feeds);
    return results[onnxSession.outputNames[0]];
}

// Post-process mask output
function postprocessMask(outputTensor, modelType, origWidth, origHeight) {
    const outputData = outputTensor.data;
    const maskHeight = outputTensor.dims[2];
    const maskWidth = outputTensor.dims[3];

    // Apply sigmoid and process according to model type
    let processedMask;

    if (modelType === 'birefnet') {
        // BiRefNet: sigmoid + min-max normalization
        const sigmoidValues = new Float32Array(maskHeight * maskWidth);
        let minVal = Infinity;
        let maxVal = -Infinity;

        for (let i = 0; i < outputData.length; i++) {
            const sigmoid = 1.0 / (1.0 + Math.exp(-outputData[i]));
            sigmoidValues[i] = sigmoid;
            minVal = Math.min(minVal, sigmoid);
            maxVal = Math.max(maxVal, sigmoid);
        }

        const range = maxVal - minVal;
        const safeRange = range < 1e-6 ? 1.0 : range;

        processedMask = new Uint8Array(maskHeight * maskWidth);
        for (let i = 0; i < sigmoidValues.length; i++) {
            const normalized = (sigmoidValues[i] - minVal) / safeRange;
            processedMask[i] = Math.round(normalized * 255);
        }
    } else {
        // U2Net, ISNet, and RMBG: sigmoid only
        processedMask = new Uint8Array(maskHeight * maskWidth);
        for (let i = 0; i < outputData.length; i++) {
            const sigmoid = 1.0 / (1.0 + Math.exp(-outputData[i]));
            processedMask[i] = Math.round(sigmoid * 255);
        }
    }

    // Resize mask back to original dimensions
    return resizeMask(processedMask, maskWidth, maskHeight, origWidth, origHeight);
}

// Resize mask using canvas
function resizeMask(maskData, maskWidth, maskHeight, targetWidth, targetHeight) {
    const canvas = document.createElement('canvas');
    canvas.width = maskWidth;
    canvas.height = maskHeight;
    const ctx = canvas.getContext('2d');

    // Create ImageData from mask
    const imageData = ctx.createImageData(maskWidth, maskHeight);
    for (let i = 0; i < maskData.length; i++) {
        const idx = i * 4;
        imageData.data[idx + 0] = maskData[i];
        imageData.data[idx + 1] = maskData[i];
        imageData.data[idx + 2] = maskData[i];
        imageData.data[idx + 3] = 255;
    }
    ctx.putImageData(imageData, 0, 0);

    // Resize using another canvas
    const resizeCanvas = document.createElement('canvas');
    resizeCanvas.width = targetWidth;
    resizeCanvas.height = targetHeight;
    const resizeCtx = resizeCanvas.getContext('2d');
    resizeCtx.drawImage(canvas, 0, 0, targetWidth, targetHeight);

    const resizedData = resizeCtx.getImageData(0, 0, targetWidth, targetHeight);
    const result = new Uint8Array(targetWidth * targetHeight);
    for (let i = 0; i < result.length; i++) {
        result[i] = resizedData.data[i * 4];
    }

    return result;
}

// Calculate mask threshold ratio from slider value
function getMaskThresholdRatio() {
    // Slider configuration: range 30-80 maps to threshold ratio 0.30-0.80
    const SLIDER_MIN = 30;
    const SLIDER_RANGE = 50;
    const RATIO_MIN = 0.30;
    const RATIO_RANGE = 0.50;
    const CURVE_EXPONENT = 1.5;  // Exponential curve for smoother control

    const thresholdSlider = document.getElementById('maskThreshold');
    const sliderValue = thresholdSlider ? parseInt(thresholdSlider.value) : 50;
    const normalized = (sliderValue - SLIDER_MIN) / SLIDER_RANGE;
    const curved = Math.pow(normalized, CURVE_EXPONENT);
    return RATIO_MIN + (curved * RATIO_RANGE);
}

// Apply mask to image (create RGBA with alpha channel)
async function applyMaskToImage(imageBytes, mask, width, height) {
    // Decode original image
    const blob = new Blob([imageBytes]);
    const imageBitmap = await createImageBitmap(blob);

    // Create canvas to extract RGB data
    const canvas = document.createElement('canvas');
    canvas.width = width;
    canvas.height = height;
    const ctx = canvas.getContext('2d');
    ctx.drawImage(imageBitmap, 0, 0, width, height);

    const imageData = ctx.getImageData(0, 0, width, height);
    const pixels = imageData.data;

    // Apply mask with configurable threshold
    const threshold = getMaskThresholdRatio() * 255; // Convert to 0-255 range

    for (let i = 0; i < mask.length; i++) {
        const alpha = mask[i] > threshold ? 255 : 0;
        pixels[i * 4 + 3] = alpha;
    }

    ctx.putImageData(imageData, 0, 0);

    // Convert to PNG bytes
    return new Promise((resolve) => {
        canvas.toBlob(async (blob) => {
            const arrayBuffer = await blob.arrayBuffer();
            resolve(new Uint8Array(arrayBuffer));
        }, 'image/png');
    });
}

// Process image with background removal
async function processImageWithRembg(imageBytes) {
    if (!onnxSession || !currentModelType) {
        throw new Error('No ONNX model loaded');
    }

    showProgressDiv('Removing background...', 0);

    const totalStart = performance.now();

    try {
        // Decode image to get dimensions
        const blob = new Blob([imageBytes]);
        const imageBitmap = await createImageBitmap(blob);
        const origWidth = imageBitmap.width;
        const origHeight = imageBitmap.height;

        console.log(`Processing image: ${origWidth}x${origHeight}`);

        // Preprocess
        showProgressDiv('Preprocessing image...', 20);
        const preprocessStart = performance.now();
        const inputTensor = await preprocessImage(imageBitmap, currentModelType);
        const preprocessTime = performance.now() - preprocessStart;
        console.log(`Preprocessing took: ${preprocessTime.toFixed(0)}ms`);

        // Run inference
        showProgressDiv('Running inference...', 40);
        const inferenceStart = performance.now();
        const outputTensor = await runOnnxInference(inputTensor);
        const inferenceTime = performance.now() - inferenceStart;
        console.log(`⚡ Inference took: ${inferenceTime.toFixed(0)}ms`);

        // Post-process mask
        showProgressDiv('Processing mask...', 70);
        const postprocessStart = performance.now();
        const mask = postprocessMask(outputTensor, currentModelType, origWidth, origHeight);
        const postprocessTime = performance.now() - postprocessStart;
        console.log(`Postprocessing took: ${postprocessTime.toFixed(0)}ms`);

        // Apply mask to create RGBA image
        showProgressDiv('Applying mask...', 85);
        const applyStart = performance.now();
        const rgbaBytes = await applyMaskToImage(imageBytes, mask, origWidth, origHeight);
        const applyTime = performance.now() - applyStart;
        console.log(`Applying mask took: ${applyTime.toFixed(0)}ms`);

        const totalTime = performance.now() - totalStart;
        console.log(`✓ Total background removal: ${totalTime.toFixed(0)}ms (${(totalTime / 1000).toFixed(1)}s)`);

        hideProgressDiv();
        return rgbaBytes;
    } catch (error) {
        hideProgressDiv();
        throw error;
    }
}

// Show status message
function showStatus(message, type = 'info') {
    statusDiv.innerHTML = `<div class="status ${type}">${message}</div>`;
}

// Preset configurations
const presets = {
    standard: { threshLow: 80, threshHigh: 160, clipLimit: 3.0, tileSize: 8 },
    strong: { threshLow: 70, threshHigh: 150, clipLimit: 4.0, tileSize: 8 },
    fine: { threshLow: 80, threshHigh: 160, clipLimit: 2.5, tileSize: 4 }
};

// Apply preset to sliders
function applyPreset(presetName) {
    if (presets[presetName]) {
        const preset = presets[presetName];
        threshLowSlider.value = preset.threshLow;
        threshLowValue.textContent = preset.threshLow;
        threshHighSlider.value = preset.threshHigh;
        threshHighValue.textContent = preset.threshHigh;
        clipLimitSlider.value = preset.clipLimit;
        clipLimitValue.textContent = preset.clipLimit.toFixed(1);
        tileSizeSlider.value = preset.tileSize;
        tileSizeValue.textContent = preset.tileSize;
    }
}

// Debounced auto-process
function scheduleAutoProcess() {
    if (!autoProcessCheckbox.checked || !currentImageBytes) {
        return;
    }

    // Clear existing timeout
    if (processingTimeout) {
        clearTimeout(processingTimeout);
    }

    // Schedule new processing after 500ms of no changes
    processingTimeout = setTimeout(() => {
        if (!isProcessing) {
            processImage();
        }
    }, 500);
}

// Load and display original image
async function loadImage(file) {
    try {
        // Read file as array buffer
        const arrayBuffer = await file.arrayBuffer();
        currentImageBytes = new Uint8Array(arrayBuffer);

        // Create object URL for display
        const blob = new Blob([currentImageBytes], { type: file.type });
        const url = URL.createObjectURL(blob);

        // Display original image
        originalWrapper.innerHTML = `<img src="${url}" alt="Original image">`;
        originalWrapper.classList.remove('empty');

        // Enable process button
        processBtn.disabled = false;

        showStatus('Image loaded successfully', 'success');
        setTimeout(() => {
            statusDiv.innerHTML = '';
        }, 2000);

        // Auto-process if enabled
        scheduleAutoProcess();
    } catch (error) {
        showStatus(`Failed to load image: ${error.message}`, 'error');
        console.error('Image loading error:', error);
    }
}

// Process image with zanbergify
async function processImage() {
    if (!wasmInitialized) {
        showStatus('WASM module not initialized', 'error');
        return;
    }

    if (!currentImageBytes) {
        showStatus('No image loaded', 'error');
        return;
    }

    try {
        isProcessing = true;
        processBtn.disabled = true;
        const startTime = performance.now();
        showStatus('Processing image...', 'info');

        // Step 1: Optionally apply background removal
        let imageBytesToProcess = currentImageBytes;
        if (enableRembgCheckbox.checked && onnxSession) {
            try {
                imageBytesToProcess = await processImageWithRembg(currentImageBytes);
            } catch (error) {
                showStatus(`Background removal failed: ${error.message}`, 'error');
                console.error('Background removal error:', error);
                return;
            }
        }

        // Step 2: Get processing parameters
        const threshLow = parseInt(threshLowSlider.value);
        const threshHigh = parseInt(threshHighSlider.value);
        const clipLimit = parseFloat(clipLimitSlider.value);
        const tileSize = parseInt(tileSizeSlider.value);

        const params = new DetailedParams(threshLow, threshHigh, clipLimit, tileSize);

        // Step 3: Get selected palette
        const paletteValue = paletteSelect.value;
        let palette;
        switch (paletteValue) {
            case 'original':
                palette = ColorPalette.original();
                break;
            case 'burgundy':
                palette = ColorPalette.burgundy();
                break;
            case 'burgundyTeal':
                palette = ColorPalette.burgundyTeal();
                break;
            case 'burgundyGold':
                palette = ColorPalette.burgundyGold();
                break;
            case 'rose':
                palette = ColorPalette.rose();
                break;
            case 'cmyk':
                palette = ColorPalette.cmyk();
                break;
            default:
                palette = ColorPalette.original();
        }

        // Step 4: Process image with posterization
        const resultBytes = ZanbergifyProcessor.processImage(
            imageBytesToProcess,
            params,
            palette
        );

        // Step 5: Create blob and display result
        const blob = new Blob([resultBytes], { type: 'image/png' });
        const url = URL.createObjectURL(blob);

        resultWrapper.innerHTML = `
            <img src="${url}" alt="Processed image">
        `;
        resultWrapper.classList.remove('empty');

        const endTime = performance.now();
        const duration = ((endTime - startTime) / 1000).toFixed(2);

        showStatus(
            `Image processed successfully in ${duration}s
            <a href="${url}" download="zanbergify-result.png" class="download-link">Download Result</a>`,
            'success'
        );
    } catch (error) {
        showStatus(`Processing failed: ${error.message}`, 'error');
        console.error('Processing error:', error);
    } finally {
        isProcessing = false;
        processBtn.disabled = false;
    }
}

// Event listeners
imageInput.addEventListener('change', (e) => {
    const file = e.target.files[0];
    if (file) {
        loadImage(file);
    }
});

processBtn.addEventListener('click', processImage);

// Preset selection
presetSelect.addEventListener('change', (e) => {
    const preset = e.target.value;
    if (preset !== 'custom') {
        applyPreset(preset);
        scheduleAutoProcess();
    }
});

// Palette change triggers auto-process
paletteSelect.addEventListener('change', () => {
    scheduleAutoProcess();
});

// Slider value display updates and auto-process
threshLowSlider.addEventListener('input', (e) => {
    threshLowValue.textContent = e.target.value;
    presetSelect.value = 'custom';
    scheduleAutoProcess();
});

threshHighSlider.addEventListener('input', (e) => {
    threshHighValue.textContent = e.target.value;
    presetSelect.value = 'custom';
    scheduleAutoProcess();
});

clipLimitSlider.addEventListener('input', (e) => {
    clipLimitValue.textContent = parseFloat(e.target.value).toFixed(1);
    presetSelect.value = 'custom';
    scheduleAutoProcess();
});

tileSizeSlider.addEventListener('input', (e) => {
    tileSizeValue.textContent = e.target.value;
    presetSelect.value = 'custom';
    scheduleAutoProcess();
});

// ========== Background Removal Event Handlers ==========

// Mask threshold slider with exponential curve for better sensitivity distribution
const maskThresholdSlider = document.getElementById('maskThreshold');
const maskThresholdValue = document.getElementById('maskThresholdValue');
if (maskThresholdSlider && maskThresholdValue) {
    maskThresholdSlider.addEventListener('input', (e) => {
        const threshold = getMaskThresholdRatio();
        maskThresholdValue.textContent = threshold.toFixed(2);
        scheduleAutoProcess();
    });
}

// Toggle background removal and auto-load model
enableRembgCheckbox.addEventListener('change', (e) => {
    rembgControls.style.display = e.target.checked ? 'block' : 'none';

    if (e.target.checked) {
        // Load model if not already loaded
        if (!onnxSession) {
            loadBundledModel().catch(error => {
                showStatus(`Failed to load model: ${error.message}`, 'error');
            });
        } else {
            scheduleAutoProcess();
        }
    } else {
        // If disabling, clear session
        onnxSession = null;
        currentModelType = null;
        scheduleAutoProcess();
    }
});

// Click model status to reload model
modelStatusDiv.addEventListener('click', () => {
    if (enableRembgCheckbox.checked) {
        loadBundledModel().catch(error => {
            showStatus(`Failed to load model: ${error.message}`, 'error');
        });
    }
});

// Initialize on page load
initWasm();
