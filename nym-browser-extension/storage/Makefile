# Nym Extension Storage - Build Configuration
# 
# This Makefile helps build the WebAssembly storage component
# and run development examples.

wasm-pack:
	@echo "🔨 Building WebAssembly package..."
	unset RUSTC_WRAPPER && wasm-pack build --target web --scope nymproject --out-dir ../../dist/wasm/extension-storage
	@echo "✅ Build complete! Output in ../../dist/wasm/extension-storage"

wasm-pack-dev:
	@echo "🔨 Building WebAssembly package (development mode)..."
	unset RUSTC_WRAPPER && wasm-pack build --dev --target web --scope nymproject --out-dir ../../dist/wasm/extension-storage
	@echo "✅ Development build complete!"


copy-wasm-to-demo:
	@echo "📁 Copying WASM files to demo directory..."
	@mkdir -p internal-dev/wasm
	@cp ../../dist/wasm/extension-storage/extension_storage.js internal-dev/wasm/
	@cp ../../dist/wasm/extension-storage/extension_storage_bg.wasm internal-dev/wasm/
	@cp ../../dist/wasm/extension-storage/extension_storage.d.ts internal-dev/wasm/
	@echo "✅ WASM files copied to internal-dev/wasm/"


clean:
	@echo "🧹 Cleaning build artifacts..."
	cargo clean
	rm -rf ../../dist/wasm/extension-storage
	rm -rf pkg
	rm -rf internal-dev/wasm
	@echo "✅ Clean complete!"

demo: wasm-pack-dev copy-wasm-to-demo
	cd internal-dev && node serve.js


check-deps:
	@echo "🔍 Checking dependencies..."
	@command -v cargo >/dev/null 2>&1 || { echo "❌ cargo is required but not installed. Please install Rust."; exit 1; }
	@command -v wasm-pack >/dev/null 2>&1 || { echo "❌ wasm-pack is required but not installed. Run: cargo install wasm-pack"; exit 1; }
	@echo "✅ All dependencies are installed!"

help:
	@echo "Nym Extension Storage Build Commands:"
	@echo ""
	@echo "📦 Building:"
	@echo "  make wasm-pack       - Build optimized WASM package for production"
	@echo "  make wasm-pack-dev   - Build WASM package with debug symbols"
	@echo "  make copy-wasm-to-demo - Copy WASM files to demo directory"
	@echo ""
	@echo "🧪 Development:"
	@echo "  make demo          - Build and serve the interactive demo (Node.js)"
	@echo "  make check-deps    - Verify all required tools are installed"
	@echo ""
	@echo "🧹 Maintenance:"
	@echo "  make clean         - Remove all build artifacts"
	@echo "  make help          - Show this help message"
	@echo ""
	@echo "💡 Quick start: make demo"


.DEFAULT_GOAL := help

.PHONY: wasm-pack wasm-pack-dev copy-wasm-to-demo clean demo check-deps help
