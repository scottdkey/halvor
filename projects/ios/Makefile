.PHONY: build clean install test

# Build the Swift bridge
build:
	./build.sh

# Clean build artifacts
clean:
	rm -rf Sources/HalvorSwiftFFI/HalvorSwiftFFI.xcframework
	rm -rf Sources/HalvorSwiftFFI/*.h
	rm -rf Sources/HalvorSwiftFFI/*.swift
	cd halvor-ffi && cargo clean

# Install Rust targets if needed
# Detect host architecture to install appropriate targets
install-targets:
	@echo "Installing Rust targets for Swift bridge..."
	@HOST_ARCH=$$(uname -m); \
	echo "Host architecture: $$HOST_ARCH"; \
	rustup target add aarch64-apple-darwin || true; \
	if [ "$$HOST_ARCH" = "x86_64" ]; then \
		echo "Installing x86_64 targets..."; \
		rustup target add x86_64-apple-darwin || true; \
		rustup target add x86_64-apple-ios || true; \
		rustup target add x86_64-apple-ios-sim || true; \
	fi; \
	rustup target add aarch64-apple-ios || true; \
	rustup target add aarch64-apple-ios-sim || true; \
	echo "✓ Rust targets installed"

# Install uniffi-bindgen
# Note: UniFFI 0.30 includes a CLI feature in the uniffi crate
# We build our own uniffi-bindgen binary from the halvor-ffi crate
install-uniffi:
	@echo "✓ No installation needed - uniffi-bindgen is built from halvor-ffi crate"
	@echo "  The build script uses: cargo run --bin uniffi-bindgen"

# Setup everything
setup: install-targets install-uniffi

# Test the package
test:
	swift test
