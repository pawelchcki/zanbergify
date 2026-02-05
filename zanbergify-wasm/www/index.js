import init, { ZanbergifyProcessor, DetailedParams, ColorPalette } from '../pkg/zanbergify_wasm.js';

let wasmInitialized = false;
let currentImageBytes = null;

// ONNX Runtime state (loaded globally via script tag)
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
        ort.env.wasm.wasmPaths = 'https://cdn.jsdelivr.net/npm/onnxruntime-web@1.20.0/dist/';

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
            return 1024;
        default:
            return 320;
    }
}

// Load bundled BiRefNet model
async function loadBundledModel() {
    try {
        await initOnnxRuntime();

        // Using U2Net for better ONNX Runtime Web compatibility
        currentModelType = 'u2net';
        const modelUrl = 'https://zanbergify-models-cdn.pawelchcki.workers.dev/u2net.onnx';
        const cacheKey = `bundled_u2net`;

        let modelData = null;

        // Check cache first
        const cached = await getCachedModel(cacheKey);

        if (cached) {
            showProgressDiv('Loading from cache...', 10);
            modelData = cached;
        } else {
            // Load from bundled file
            showProgressDiv('Loading model...', 10);
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
                    showProgressDiv(`Loading model... ${Math.round(loaded / 1024 / 1024)}MB / ${Math.round(total / 1024 / 1024)}MB`, progress);
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
        showProgressDiv('Initializing BiRefNet model...', 90);

        console.log('Creating ONNX session...');

        let usedBackend = 'wasm';  // Default to WASM
        let sessionOptions;

        // Try multiple WebGPU configurations
        const webgpuConfigs = [
            // Config 1: Simple string
            {
                executionProviders: ['webgpu'],
                graphOptimizationLevel: 'basic'
            },
            // Config 2: With device options
            {
                executionProviders: [{
                    name: 'webgpu',
                    preferredLayout: 'NHWC'
                }],
                graphOptimizationLevel: 'basic'
            }
        ];

        let sessionCreated = false;

        // Try WebGPU if available
        if (navigator.gpu) {
            for (let i = 0; i < webgpuConfigs.length && !sessionCreated; i++) {
                try {
                    console.log(`Attempting WebGPU config ${i + 1}...`);
                    onnxSession = await ort.InferenceSession.create(modelData, webgpuConfigs[i]);
                    usedBackend = 'webgpu';
                    sessionCreated = true;
                    console.log('✓ Session created with WebGPU backend');
                    break;
                } catch (err) {
                    console.warn(`WebGPU config ${i + 1} failed:`, err.message);
                }
            }
        } else {
            console.log('WebGPU not available, using WASM');
        }

        // Fallback to WASM if WebGPU failed
        if (!sessionCreated) {
            console.log('Falling back to WASM backend...');
            sessionOptions = {
                executionProviders: ['wasm'],
                graphOptimizationLevel: 'basic'
            };

            try {
                onnxSession = await ort.InferenceSession.create(modelData, sessionOptions);
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

        modelStatusDiv.textContent = `✓ BiRefNet ready (1024x1024, ${backendText} ${backendEmoji})`;
        modelStatusDiv.style.background = '#e8f5e9';
        modelStatusDiv.style.color = '#2e7d32';
        modelStatusDiv.style.cursor = 'default';

        console.log(`Model loaded successfully using ${backendText} backend`);

        // Trigger reprocessing if image is loaded
        scheduleAutoProcess();
    } catch (error) {
        hideProgressDiv();
        modelStatusDiv.textContent = `Failed to load model: ${error.message}`;
        modelStatusDiv.style.background = '#ffebee';
        modelStatusDiv.style.color = '#c62828';
        throw error;
    }
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
        // U2Net and ISNet: sigmoid only
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

    // Apply mask with configurable threshold (exponential curve)
    const thresholdSlider = document.getElementById('maskThreshold');
    const sliderValue = thresholdSlider ? parseInt(thresholdSlider.value) : 50;
    const normalized = (sliderValue - 30) / 50; // Normalize to 0-1
    const curved = Math.pow(normalized, 1.5); // Apply exponential curve
    const thresholdRatio = 0.30 + (curved * 0.50); // Map to 0.30-0.80
    const threshold = thresholdRatio * 255; // Convert to 0-255 range

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
        const sliderValue = parseInt(e.target.value);
        // Map 30-80 range with slight exponential curve for smoother control
        const normalized = (sliderValue - 30) / 50; // 0-1 range
        const curved = Math.pow(normalized, 1.5); // Exponential curve
        const threshold = 0.30 + (curved * 0.50); // Map to 0.30-0.80 range
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
