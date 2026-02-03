import init, { ZanbergifyProcessor, DetailedParams, ColorPalette } from '../pkg/zanbergify_wasm.js';

let wasmInitialized = false;
let currentImageBytes = null;

// DOM elements
const imageInput = document.getElementById('imageInput');
const presetSelect = document.getElementById('presetSelect');
const paletteSelect = document.getElementById('paletteSelect');
const autoProcessCheckbox = document.getElementById('autoProcess');
const processBtn = document.getElementById('processBtn');
const statusDiv = document.getElementById('status');
const originalWrapper = document.getElementById('originalWrapper');
const resultWrapper = document.getElementById('resultWrapper');

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

        // Get slider values and create custom params
        const threshLow = parseInt(threshLowSlider.value);
        const threshHigh = parseInt(threshHighSlider.value);
        const clipLimit = parseFloat(clipLimitSlider.value);
        const tileSize = parseInt(tileSizeSlider.value);

        const params = new DetailedParams(threshLow, threshHigh, clipLimit, tileSize);

        // Get selected palette
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

        // Process image
        const resultBytes = ZanbergifyProcessor.processImage(
            currentImageBytes,
            params,
            palette
        );

        // Create blob and display result
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

// Initialize on page load
initWasm();
