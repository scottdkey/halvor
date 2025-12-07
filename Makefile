.PHONY: install dev build clean setup-dev build-linux build-macos build-windows build-vpn build-all help setup-linux-cross-compile build-linux-target

# Default target
help:
	@echo "Available targets:"
	@echo "  make build          - Build for current platform (release)"
	@echo "  make build linux    - Build all Linux targets"
	@echo "  make build macos    - Build all macOS targets"
	@echo "  make build windows  - Build Windows target"
	@echo "  make build vpn      - Build VPN Docker container"
	@echo "  make build all      - Build all CLI targets"
	@echo ""
	@echo "Alternative syntax:"
	@echo "  make build-linux    - Build all Linux targets"
	@echo "  make build-macos    - Build all macOS targets"
	@echo "  make build-windows - Build Windows target"
	@echo "  make build-vpn     - Build VPN Docker container"
	@echo "  make build-all     - Build all CLI targets"
	@echo ""
	@echo "Other targets:"
	@echo "  make install        - Install hal globally"
	@echo "  make dev            - Start development mode (watch)"
	@echo "  make clean          - Clean build artifacts"

# Install hal globally
install:
	cargo install --path . --force

# Setup development dependencies
setup-dev:
	@echo "Installing cargo-watch for development..."
	cargo install cargo-watch

# Development mode: watch for changes and auto-install
dev: setup-dev
	@echo "Starting development mode..."
	@echo "Watching for changes and auto-installing..."
	@echo "Press Ctrl+C to stop"
	cargo watch -x 'install --path . --force'

# Build the binary for current platform
# When used with platform targets, skip default build
build:
	@# Check if any platform-specific targets are in the command line
	@if echo "$(MAKECMDGOALS)" | grep -qE "(linux|macos|windows|vpn|pia-vpn|all)"; then \
		: ; \
	else \
		if ! command -v cargo >/dev/null 2>&1; then \
			echo "Cargo not found. Installing Rust..."; \
			curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable; \
		fi; \
		if ! command -v cc >/dev/null 2>&1 && ! command -v gcc >/dev/null 2>&1; then \
			echo "C compiler not found. Installing build dependencies..."; \
			if command -v apt-get >/dev/null 2>&1; then \
				if sudo -n apt-get update >/dev/null 2>&1; then \
					sudo apt-get install -y build-essential; \
				else \
					echo "Error: Passwordless sudo required to install build-essential"; \
					echo "Run: sudo apt-get update && sudo apt-get install -y build-essential"; \
					exit 1; \
				fi; \
			elif command -v yum >/dev/null 2>&1; then \
				if sudo -n yum list installed gcc >/dev/null 2>&1; then \
					sudo yum install -y gcc gcc-c++ make; \
				else \
					echo "Error: Passwordless sudo required to install gcc"; \
					echo "Run: sudo yum install -y gcc gcc-c++ make"; \
					exit 1; \
				fi; \
			elif command -v dnf >/dev/null 2>&1; then \
				if sudo -n dnf list installed gcc >/dev/null 2>&1; then \
					sudo dnf install -y gcc gcc-c++ make; \
				else \
					echo "Error: Passwordless sudo required to install gcc"; \
					echo "Run: sudo dnf install -y gcc gcc-c++ make"; \
					exit 1; \
				fi; \
			fi; \
		fi; \
		export PATH="$$HOME/.cargo/bin:$$PATH" && cargo build --release; \
	fi

# Detect host platform
HOST_OS := $(shell uname -s)
HOST_ARCH := $(shell uname -m)

# Build all Linux targets
build-linux:
	@if ! command -v cargo >/dev/null 2>&1; then \
		echo "Cargo not found. Installing Rust..."; \
		curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable; \
		. $$HOME/.cargo/env && cargo --version; \
	fi
	@. $$HOME/.cargo/env 2>/dev/null || true; \
	echo "Building Linux targets..."; \
	echo "Host platform: $(HOST_OS) $(HOST_ARCH)"; \
	if [ "$(HOST_OS)" = "Darwin" ]; then \
		echo "Detected macOS host - setting up cross-compilation..."; \
		$(MAKE) setup-linux-cross-compile; \
	fi; \
	echo "Building x86_64-unknown-linux-gnu..."; \
	$(MAKE) build-linux-target TARGET=x86_64-unknown-linux-gnu; \
	echo "Building x86_64-unknown-linux-musl..."; \
	$(MAKE) build-linux-target TARGET=x86_64-unknown-linux-musl; \
	echo "Building aarch64-unknown-linux-gnu..."; \
	$(MAKE) build-linux-target TARGET=aarch64-unknown-linux-gnu; \
	echo "✓ All Linux builds complete"

# Setup cross-compilation tools for Linux on macOS
setup-linux-cross-compile:
	@echo "Checking cross-compilation toolchains..."
	@echo "Note: Cross-compiling from macOS to Linux can be complex."
	@echo "For reliable builds, consider using Docker or building on a Linux host."
	@echo ""
	@if ! rustup target list --installed | grep -q "x86_64-unknown-linux-gnu"; then \
		echo "Installing Rust target: x86_64-unknown-linux-gnu..."; \
		rustup target add x86_64-unknown-linux-gnu; \
	fi
	@if ! rustup target list --installed | grep -q "x86_64-unknown-linux-musl"; then \
		echo "Installing Rust target: x86_64-unknown-linux-musl..."; \
		rustup target add x86_64-unknown-linux-musl; \
	fi
	@if ! rustup target list --installed | grep -q "aarch64-unknown-linux-gnu"; then \
		echo "Installing Rust target: aarch64-unknown-linux-gnu..."; \
		rustup target add aarch64-unknown-linux-gnu; \
	fi
	@echo ""
	@echo "Checking for cargo-zigbuild..."
	@if ! command -v cargo-zigbuild >/dev/null 2>&1; then \
		echo "cargo-zigbuild not found. Installing..."; \
		cargo install cargo-zigbuild || echo "Failed to install cargo-zigbuild. You may need to install it manually: cargo install cargo-zigbuild"; \
	fi
	@echo "Checking for zig (required by cargo-zigbuild)..."
	@if ! command -v zig >/dev/null 2>&1; then \
		echo "zig not found. Attempting to install..."; \
		if command -v brew >/dev/null 2>&1; then \
			echo "Installing zig via Homebrew..."; \
			brew install zig || echo "Failed to install zig via Homebrew. You may need to install it manually: brew install zig"; \
		else \
			echo "Homebrew not found. Please install zig manually:"; \
			echo "  macOS: brew install zig"; \
			echo "  Linux: See https://ziglang.org/download/"; \
		fi; \
	fi

# Build a specific Linux target with proper cross-compilation setup
build-linux-target:
	@. $$HOME/.cargo/env 2>/dev/null || true; \
	if [ "$(HOST_OS)" = "Darwin" ]; then \
		echo "Cross-compiling from macOS to $(TARGET)..."; \
		unset MACOSX_DEPLOYMENT_TARGET || true; \
		if [ "$(TARGET)" = "x86_64-unknown-linux-gnu" ]; then \
			if ! command -v cargo-zigbuild >/dev/null 2>&1; then \
				echo "cargo-zigbuild not found. Installing..."; \
				cargo install cargo-zigbuild || (echo "Failed to install cargo-zigbuild. Please install manually: cargo install cargo-zigbuild" && exit 1); \
			fi; \
			if ! command -v zig >/dev/null 2>&1; then \
				echo "zig not found. Installing zig..."; \
				if command -v brew >/dev/null 2>&1; then \
					brew install zig || (echo "Failed to install zig. Please install manually: brew install zig" && exit 1); \
				else \
					echo "Homebrew not found. Please install zig manually: brew install zig"; \
					exit 1; \
				fi; \
			fi; \
			echo "Using cargo-zigbuild for cross-compilation..."; \
			cargo zigbuild --release --target $(TARGET); \
		elif [ "$(TARGET)" = "x86_64-unknown-linux-musl" ]; then \
			if ! command -v cargo-zigbuild >/dev/null 2>&1; then \
				echo "cargo-zigbuild not found. Installing..."; \
				cargo install cargo-zigbuild || (echo "Failed to install cargo-zigbuild. Please install manually: cargo install cargo-zigbuild" && exit 1); \
			fi; \
			if ! command -v zig >/dev/null 2>&1; then \
				echo "zig not found. Installing zig..."; \
				if command -v brew >/dev/null 2>&1; then \
					brew install zig || (echo "Failed to install zig. Please install manually: brew install zig" && exit 1); \
				else \
					echo "Homebrew not found. Please install zig manually: brew install zig"; \
					exit 1; \
				fi; \
			fi; \
			echo "Using cargo-zigbuild for cross-compilation..."; \
			cargo zigbuild --release --target $(TARGET); \
		elif [ "$(TARGET)" = "aarch64-unknown-linux-gnu" ]; then \
			if ! command -v cargo-zigbuild >/dev/null 2>&1; then \
				echo "cargo-zigbuild not found. Installing..."; \
				cargo install cargo-zigbuild || (echo "Failed to install cargo-zigbuild. Please install manually: cargo install cargo-zigbuild" && exit 1); \
			fi; \
			if ! command -v zig >/dev/null 2>&1; then \
				echo "zig not found. Installing zig..."; \
				if command -v brew >/dev/null 2>&1; then \
					brew install zig || (echo "Failed to install zig. Please install manually: brew install zig" && exit 1); \
				else \
					echo "Homebrew not found. Please install zig manually: brew install zig"; \
					exit 1; \
				fi; \
			fi; \
			echo "Using cargo-zigbuild for cross-compilation..."; \
			cargo zigbuild --release --target $(TARGET); \
		else \
			echo "Unknown target: $(TARGET)"; \
			exit 1; \
		fi \
	else \
		echo "Building natively on Linux for $(TARGET)..."; \
		cargo build --release --target $(TARGET); \
	fi

# Build all macOS targets
build-macos:
	@echo "Building macOS targets..."
	@echo "Building x86_64-apple-darwin..."
	@cargo build --release --target x86_64-apple-darwin
	@echo "Building aarch64-apple-darwin..."
	@cargo build --release --target aarch64-apple-darwin
	@echo "✓ All macOS builds complete"

# Build Windows target
build-windows:
	@if ! command -v cargo >/dev/null 2>&1; then \
		echo "Cargo not found. Installing Rust..."; \
		curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable; \
		. $$HOME/.cargo/env && cargo --version; \
	fi
	@. $$HOME/.cargo/env 2>/dev/null || true; \
	echo "Building Windows target..."; \
	if ! rustup target list --installed | grep -q "x86_64-pc-windows-msvc"; then \
		echo "Installing Rust target: x86_64-pc-windows-msvc..."; \
		rustup target add x86_64-pc-windows-msvc; \
	fi
	@if [ "$(HOST_OS)" = "Darwin" ] || [ "$(HOST_OS)" = "Linux" ]; then \
		echo "Cross-compiling from $(HOST_OS) to Windows..."; \
		if ! command -v cargo-xwin >/dev/null 2>&1; then \
			echo "cargo-xwin not found. Installing..."; \
			cargo install cargo-xwin || (echo "Failed to install cargo-xwin. Please install manually: cargo install cargo-xwin" && exit 1); \
		fi; \
		if ! rustup component list --installed | grep -q "llvm-tools-preview"; then \
			echo "Installing llvm-tools-preview component..."; \
			rustup component add llvm-tools-preview; \
		fi; \
		echo "Building x86_64-pc-windows-msvc using cargo-xwin..."; \
		cargo xwin build --release --target x86_64-pc-windows-msvc; \
	else \
		echo "Building x86_64-pc-windows-msvc natively..."; \
		cargo build --release --target x86_64-pc-windows-msvc; \
	fi
	@echo "✓ Windows build complete"

# Build VPN Docker container
build-vpn:
	@echo "Building VPN Docker container..."
	@if [ ! -d "openvpn-container" ]; then \
		echo "Error: openvpn-container directory not found"; \
		exit 1; \
	fi
	@cd openvpn-container && docker build -t vpn:latest -f Dockerfile .
	@echo "✓ VPN container built successfully"
	@echo "  Image: vpn:latest"

# Alias for build-vpn (pia-vpn)
build-pia-vpn: build-vpn

# Build all CLI targets (Linux, macOS, Windows)
build-all: build-linux build-macos build-windows
	@echo "✓ All CLI builds complete"

# Handle "make build <target>" syntax
# When user runs "make build linux", Make executes both "build" and "linux"
# We make these targets call the actual build targets
linux: build-linux
macos: build-macos
windows: build-windows
vpn: build-vpn
pia-vpn: build-vpn
all: build-all

# Clean build artifacts
clean:
	cargo clean
	@echo "✓ Build artifacts cleaned"
