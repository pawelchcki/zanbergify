# Zanbergify WASM Quick Start

## 🚀 Get Started in 3 Steps

### 1. Build the WASM Module

```bash
cd rust/zanbergify-wasm
wasm-pack build --target web --release
```

This creates a `pkg/` directory with:
- `zanbergify_wasm_bg.wasm` (2.4 MB) - The WebAssembly binary
- `zanbergify_wasm.js` (16 KB) - JavaScript bindings
- `zanbergify_wasm.d.ts` - TypeScript definitions

### 2. Test the Demo Page

```bash
# From rust/zanbergify-wasm directory
python3 -m http.server 8080 --directory www

# Open in browser: http://localhost:8080
```

The demo page lets you:
- Upload any image (PNG, JPEG, WebP, BMP)
- Choose processing preset (standard, strong, or fine)
- Choose color palette (6 options)
- See before/after comparison
- Download the result

### 3. Integrate Into Your Project

#### Minimal Example

```html
<!DOCTYPE html>
<html>
<body>
    <input type="file" id="input" accept="image/*">
    <img id="output">
    <script type="module">
        import init, { ZanbergifyProcessor, DetailedParams, ColorPalette }
            from './pkg/zanbergify_wasm.js';

        await init();

        document.getElementById('input').onchange = async (e) => {
            const file = e.target.files[0];
            const bytes = new Uint8Array(await file.arrayBuffer());

            const result = ZanbergifyProcessor.processImage(
                bytes,
                DetailedParams.detailedStandard(),
                ColorPalette.original()
            );

            const blob = new Blob([result], { type: 'image/png' });
            document.getElementById('output').src = URL.createObjectURL(blob);
        };
    </script>
</body>
</html>
```

## 📖 Next Steps

- **Learn more:** Read [USAGE.md](USAGE.md) for detailed API documentation
- **See examples:** Check [www/index.js](www/index.js) for a complete implementation
- **Understand internals:** Read [IMPLEMENTATION_SUMMARY.md](IMPLEMENTATION_SUMMARY.md)

## 🎨 Available Options

### Presets
- `DetailedParams.detailedStandard()` - Balanced (recommended)
- `DetailedParams.detailedStrong()` - High contrast
- `DetailedParams.detailedFine()` - Fine details

### Color Palettes
- `ColorPalette.original()` - Black/Magenta/Gold
- `ColorPalette.burgundy()` - Black/Burgundy/White
- `ColorPalette.burgundyTeal()` - Burgundy/Teal
- `ColorPalette.burgundyGold()` - Burgundy/Gold
- `ColorPalette.rose()` - Rose monochrome
- `ColorPalette.cmyk()` - Cyan/Magenta

### Custom Colors
```javascript
// Create your own palette with hex colors
const palette = new ColorPalette("000000", "FF1493", "FFD700");
```

## ⚡ Performance Tips

1. **Show loading indicator** - Processing can take 1-3 seconds for large images
2. **Use Web Workers** - Keep UI responsive during processing
3. **Enable compression** - Serve .wasm with gzip/brotli (30-40% smaller)
4. **Lazy load** - Only load WASM when user needs it

## 🐛 Common Issues

**"Module not found"**
- Make sure you built with `wasm-pack build --target web`
- Check the import path points to `pkg/zanbergify_wasm.js`

**"Failed to load WASM"**
- Must serve over HTTP/HTTPS (not file://)
- Check browser console for errors

**Slow performance**
- Expected for large images (2000x2000+)
- Consider resizing before processing
- Use Web Workers for background processing

## 📦 Bundle Size

- Uncompressed: 2.4 MB
- With gzip: ~700-900 KB
- With brotli: ~600-800 KB

The size includes full image format support (PNG, JPEG, WebP, BMP, etc.).

## 🌐 Browser Support

Works in all modern browsers:
- Chrome 90+
- Firefox 88+
- Safari 14+
- Edge 90+

Requires WebAssembly and ES6 modules support.

## 🎯 What's Next?

The WASM module is production-ready! Consider:

1. **Optimization:** Reduce bundle size by stripping unused image formats
2. **Distribution:** Publish to npm for easy installation
3. **Features:** Add comic pipeline variant, batch processing
4. **Performance:** Multi-threading with Web Workers
5. **Demo:** Create a hosted demo website

Enjoy transforming images into posterized art! 🎨
