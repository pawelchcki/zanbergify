#!/bin/bash

# Build and Deploy Zanbergify WASM to Cloudflare Pages
set -e

echo "🔨 Building WASM package..."
wasm-pack build --target web --release

echo ""
echo "🚀 Deploying to Cloudflare Pages..."
./deploy-to-cloudflare.sh

echo ""
echo "✅ Build and deployment complete!"
