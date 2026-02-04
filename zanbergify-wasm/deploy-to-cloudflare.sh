#!/bin/bash

# Deploy Zanbergify WASM to Cloudflare Pages
set -e

echo "🚀 Deploying Zanbergify WASM to Cloudflare Pages..."

# Configuration
PROJECT_NAME="zanbergify-wasm"
DEPLOY_DIR="./deploy"

# Clean and create deployment directory
echo "📦 Preparing deployment directory..."
rm -rf "$DEPLOY_DIR"
mkdir -p "$DEPLOY_DIR"

# Copy www files
echo "📋 Copying web app files..."
cp www/index.html "$DEPLOY_DIR/"
cp www/_headers "$DEPLOY_DIR/" 2>/dev/null || true

# Fix import path in index.js for deployment
echo "🔧 Fixing import paths..."
sed 's|../pkg/|./pkg/|g' www/index.js > "$DEPLOY_DIR/index.js"

# Create pkg directory and copy WASM files
echo "🦀 Copying WASM package..."
mkdir -p "$DEPLOY_DIR/pkg"
cp pkg/zanbergify_wasm.js "$DEPLOY_DIR/pkg/"
cp pkg/zanbergify_wasm_bg.wasm "$DEPLOY_DIR/pkg/"
cp pkg/zanbergify_wasm.d.ts "$DEPLOY_DIR/pkg/" 2>/dev/null || true
cp pkg/zanbergify_wasm_bg.wasm.d.ts "$DEPLOY_DIR/pkg/" 2>/dev/null || true

# Copy models directory if it exists
if [ -d "www/models" ]; then
  echo "🤖 Copying ML models..."
  mkdir -p "$DEPLOY_DIR/models"
  cp -r www/models/* "$DEPLOY_DIR/models/" 2>/dev/null || echo "⚠️  Warning: No models found in www/models/"
else
  echo "⚠️  Warning: www/models directory not found"
fi

# Deploy to Cloudflare Pages
echo "☁️  Deploying to Cloudflare Pages..."
wrangler pages deploy "$DEPLOY_DIR" --project-name="$PROJECT_NAME" --branch=main --commit-dirty=true

echo "✅ Deployment complete!"
echo "🌐 Your app should be available at: https://$PROJECT_NAME.pages.dev"
