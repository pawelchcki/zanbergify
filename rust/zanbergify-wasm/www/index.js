import init, { ZanbergifyProcessor, DetailedParams, ColorPalette } from '../pkg/zanbergify_wasm.js';

let wasmInitialized = false;
let currentImageBytes = null;

// DOM elements
const imageInput = document.getElementById('imageInput');
const presetSelect = document.getElementById('presetSelect');
const paletteSelect = document.getElementById('paletteSelect');
const processBtn = document.getElementById('processBtn');
const statusDiv = document.getElementById('status');
const originalWrapper = document.getElementById('originalWrapper');
const resultWrapper = document.getElementById('resultWrapper');

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
        processBtn.disabled = true;
        const startTime = performance.now();
        showStatus('Processing image...', 'info');

        // Get selected preset
        const presetValue = presetSelect.value;
        let params;
        switch (presetValue) {
            case 'standard':
                params = DetailedParams.detailedStandard();
                break;
            case 'strong':
                params = DetailedParams.detailedStrong();
                break;
            case 'fine':
                params = DetailedParams.detailedFine();
                break;
            default:
                params = DetailedParams.detailedStandard();
        }

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

// Initialize on page load
initWasm();
