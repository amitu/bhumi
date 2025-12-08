#!/bin/bash
# Build bhumi-web for WASM deployment

echo "🚀 Building Bhumi Web..."

# Install required tools if not present
if ! command -v wasm-pack &> /dev/null; then
    echo "📦 Installing wasm-pack..."
    cargo install wasm-pack
fi

# Build for web
echo "🔧 Building WASM package..."
wasm-pack build --target web --out-dir pkg

# Create deployment directory
mkdir -p dist
cp index.html dist/
cp -r pkg dist/

echo "✅ Build complete!"
echo "📱 To test locally:"
echo "   cd dist"
echo "   python3 -m http.server 8000"
echo "   Open: http://localhost:8000"
echo ""
echo "🎮 Ready for deployment to amitu.com/bhumi"
echo "🎯 EvoFox One controller support included!"