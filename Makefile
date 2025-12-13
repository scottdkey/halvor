
# Installation and setup
.PHONY: install install-rust install-rust-targets install-rust-deps install-swift install-swift-deps install-android install-android-deps install-web install-web-deps install-tools help

# Default target
help:
	@echo "Halvor Multi-Platform Build System"
	@echo ""
	@echo "Installation:"
	@echo "  make install              - Install all dependencies (Rust, Swift, Android, Web)"
	@echo "  make install-rust         - Install Rust toolchain"
	@echo "  make install-rust-targets - Install all Rust cross-compilation targets"
	@echo "  make install-swift        - Install Swift/Xcode dependencies"
	@echo "  make install-android      - Install Android dependencies"
	@echo "  make install-web          - Install Web dependencies (Node.js, wasm-pack)"
	@echo "  make install-tools         - Install development tools (Docker, Fastlane)"
	@echo "  make install-cli          - Build and install CLI to system"
	@echo ""
	@echo "Build targets (use 'halvor build' commands):"
	@echo "  halvor build ios          - Build iOS Swift app"
	@echo "  halvor build mac          - Build macOS Swift app"
	@echo "  halvor build android      - Build Android library and app"
	@echo "  halvor build web          - Build WASM module and web app"
	@echo ""
	@echo "Development (use 'halvor dev' commands):"
	@echo "  halvor dev mac            - macOS development with hot reload"
	@echo "  halvor dev ios            - iOS development with simulator"
	@echo "  halvor dev web            - Web development with hot reload (Docker)"
	@echo "  halvor dev web --bare-metal - Web development (Rust server + Svelte dev)"
	@echo "  halvor dev web --prod     - Web production mode (Docker)"
	@echo "  halvor dev cli            - CLI development mode with watch (auto-rebuild on changes)"


# Main install target - installs all dependencies
install: install-rust install-rust-targets install-rust-deps install-swift install-swift-deps install-android install-android-deps install-web install-web-deps install-tools
	@echo ""
	@echo "✓ All dependencies installed!"
	@echo "You can now run: halvor build ios, halvor build mac, halvor build android, or halvor build web"

# Install CLI to system
.PHONY: install-cli
install-cli:
	@echo "Building and installing CLI to system..."
	@cargo build --release --bin halvor
	@cargo install --path . --bin halvor --force
	@echo "✓ CLI installed to system (available as 'halvor')"

# Install Rust toolchain
install-rust:
	@echo "Installing Rust toolchain..."
	@if ! command -v cargo >/dev/null 2>&1; then \
		echo "Rust not found. Installing via rustup..."; \
		curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable; \
		. $$HOME/.cargo/env && cargo --version; \
		echo "✓ Rust installed"; \
	else \
		echo "✓ Rust already installed: $$(cargo --version)"; \
	fi

# Install all required Rust targets
install-rust-targets: install-rust
	@echo "Installing Rust targets for all platforms..."
	@. $$HOME/.cargo/env 2>/dev/null || true; \
	echo "Installing macOS targets..."; \
	rustup target add aarch64-apple-darwin || true; \
	rustup target add x86_64-apple-darwin || true; \
	echo "Installing iOS targets..."; \
	rustup target add aarch64-apple-ios || true; \
	rustup target add aarch64-apple-ios-sim || true; \
	rustup target add x86_64-apple-ios || true; \
	echo "Installing Android targets..."; \
	rustup target add aarch64-linux-android || true; \
	rustup target add armv7-linux-androideabi || true; \
	rustup target add i686-linux-android || true; \
	rustup target add x86_64-linux-android || true; \
	echo "Installing Linux targets (for cross-compilation)..."; \
	rustup target add x86_64-unknown-linux-gnu || true; \
	rustup target add aarch64-unknown-linux-gnu || true; \
	rustup target add x86_64-unknown-linux-musl || true; \
	rustup target add aarch64-unknown-linux-musl || true; \
	echo "Installing Windows targets (for cross-compilation)..."; \
	rustup target add x86_64-pc-windows-msvc || true; \
	rustup target add aarch64-pc-windows-msvc || true; \
	echo "Installing WASM target..."; \
	rustup target add wasm32-unknown-unknown || true; \
	echo "✓ All Rust targets installed"

# Install Rust crate dependencies
install-rust-deps: install-rust
	@echo "Installing Rust crate dependencies..."
	@. $$HOME/.cargo/env 2>/dev/null || true; \
	echo "Installing cross for cross-compilation..."; \
	if command -v cross >/dev/null 2>&1; then \
		echo "Upgrading cross to latest version..."; \
		cargo install cross --force || echo "⚠️  Failed to upgrade cross"; \
	else \
		echo "Installing cross..."; \
		cargo install cross || echo "⚠️  Failed to install cross"; \
	fi; \
	echo "✓ cross installed: $$(cross --version 2>&1 | head -1)"; \
	echo "Fetching dependencies for main crate..."; \
	cargo fetch || echo "⚠️  Failed to fetch main crate dependencies"; \
	if [ -d "halvor-swift/halvor-ffi" ]; then \
		echo "Fetching dependencies for halvor-ffi..."; \
		cd halvor-swift/halvor-ffi && cargo fetch || echo "⚠️  Failed to fetch halvor-ffi dependencies"; \
	fi; \
	if [ -d "halvor-swift/halvor-ffi-macro" ]; then \
		echo "Fetching dependencies for halvor-ffi-macro..."; \
		cd halvor-swift/halvor-ffi-macro && cargo fetch || echo "⚠️  Failed to fetch halvor-ffi-macro dependencies"; \
	fi; \
	if [ -d "halvor-swift/halvor-ffi-jni" ]; then \
		echo "Fetching dependencies for halvor-ffi-jni..."; \
		cd halvor-swift/halvor-ffi-jni && cargo fetch || echo "⚠️  Failed to fetch halvor-ffi-jni dependencies"; \
	fi; \
	if [ -d "halvor-swift/halvor-ffi-wasm" ]; then \
		echo "Fetching dependencies for halvor-ffi-wasm..."; \
		cd halvor-swift/halvor-ffi-wasm && cargo fetch || echo "⚠️  Failed to fetch halvor-ffi-wasm dependencies"; \
	fi; \
	echo "✓ Rust dependencies installed"

# Install Swift/Xcode dependencies
install-swift:
	@echo "Checking Swift/Xcode dependencies..."
	@if ! command -v swift >/dev/null 2>&1; then \
		echo "⚠️  Swift not found. Please install Xcode from the App Store."; \
		echo "   After installing, run: sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer"; \
	else \
		echo "✓ Swift installed: $$(swift --version | head -1)"; \
	fi; \
	if ! command -v xcodegen >/dev/null 2>&1; then \
		echo "Installing xcodegen..."; \
		if command -v brew >/dev/null 2>&1; then \
			brew install xcodegen || echo "⚠️  Failed to install xcodegen. Install manually: brew install xcodegen"; \
		else \
			echo "⚠️  Homebrew not found. Install xcodegen manually: brew install xcodegen"; \
		fi; \
	else \
		echo "✓ xcodegen installed"; \
	fi; \
	if ! command -v cargo-watch >/dev/null 2>&1; then \
		echo "Installing cargo-watch..."; \
		. $$HOME/.cargo/env 2>/dev/null || true; \
		cargo install cargo-watch || echo "⚠️  Failed to install cargo-watch"; \
	else \
		echo "✓ cargo-watch installed"; \
	fi

# Install Swift package dependencies
install-swift-deps: install-swift
	@echo "Installing Swift package dependencies..."
	@if [ -d "halvor-swift" ]; then \
		cd halvor-swift && \
		if command -v swift >/dev/null 2>&1; then \
			echo "Resolving Swift package dependencies..."; \
			swift package resolve || echo "⚠️  Failed to resolve Swift package dependencies"; \
			echo "✓ Swift package dependencies resolved"; \
		else \
			echo "⚠️  Swift not found, skipping Swift package resolution"; \
		fi; \
	fi

# Install Android dependencies
install-android:
	@echo "Checking Android dependencies..."
	@if [ -z "$$ANDROID_NDK_HOME" ] && [ -z "$$ANDROID_NDK_ROOT" ]; then \
		echo "⚠️  Android NDK not found. Please set ANDROID_NDK_HOME or ANDROID_NDK_ROOT"; \
		echo "   Install via Android Studio SDK Manager or download from:"; \
		echo "   https://developer.android.com/ndk/downloads"; \
	else \
		echo "✓ Android NDK found: $$ANDROID_NDK_HOME$$ANDROID_NDK_ROOT"; \
	fi; \
	if ! command -v java >/dev/null 2>&1; then \
		echo "⚠️  Java not found. Android builds require Java 17+"; \
		echo "   Install via: brew install openjdk@17 (macOS) or your package manager"; \
	else \
		echo "✓ Java installed: $$(java -version 2>&1 | head -1)"; \
	fi; \
	if [ -d "halvor-android" ]; then \
		echo "Checking Gradle wrapper..."; \
		cd halvor-android && chmod +x gradlew 2>/dev/null || true; \
	fi

# Install Android Gradle dependencies
install-android-deps: install-android
	@echo "Installing Android Gradle dependencies..."
	@if [ -d "halvor-android" ]; then \
		cd halvor-android && \
		if [ -f "gradlew" ]; then \
			echo "Downloading Gradle and dependencies..."; \
			chmod +x gradlew && \
			./gradlew --no-daemon tasks --all >/dev/null 2>&1 || ./gradlew --no-daemon build --dry-run || echo "⚠️  Failed to download Gradle dependencies"; \
			echo "✓ Android Gradle dependencies installed"; \
		else \
			echo "⚠️  Gradle wrapper not found in halvor-android"; \
		fi; \
	fi

# Install Web dependencies
install-web: install-rust
	@echo "Installing Web dependencies..."
	@if ! command -v node >/dev/null 2>&1; then \
		echo "⚠️  Node.js not found. Installing via Homebrew..."; \
		if command -v brew >/dev/null 2>&1; then \
			brew install node || echo "⚠️  Failed to install Node.js. Install manually: brew install node"; \
		else \
			echo "⚠️  Homebrew not found. Install Node.js manually from: https://nodejs.org/"; \
		fi; \
	else \
		echo "✓ Node.js installed: $$(node --version)"; \
	fi; \
	if ! command -v npm >/dev/null 2>&1; then \
		echo "⚠️  npm not found. This should come with Node.js."; \
	else \
		echo "✓ npm installed: $$(npm --version)"; \
		echo "✓ npm installed: $$(npm --version)"; \
	fi; \
	if ! command -v wasm-pack >/dev/null 2>&1; then \
		echo "Installing wasm-pack..."; \
		. $$HOME/.cargo/env 2>/dev/null || true; \
		cargo install wasm-pack || echo "⚠️  Failed to install wasm-pack"; \
	else \
		echo "✓ wasm-pack installed: $$(wasm-pack --version)"; \
	fi

# Install npm/web dependencies
install-web-deps: install-web
	@echo "Installing npm dependencies for web app..."
	@if [ -d "halvor-web" ]; then \
		cd halvor-web && \
		if command -v npm >/dev/null 2>&1; then \
			echo "Running npm install..."; \
			npm install || echo "⚠️  Failed to install npm dependencies"; \
			echo "Running svelte-kit sync to initialize SvelteKit..."; \
			npx svelte-kit sync || echo "⚠️  Failed to run svelte-kit sync"; \
			echo "✓ npm dependencies installed"; \
		else \
			echo "⚠️  npm not found, skipping npm install"; \
		fi; \
	fi

# Install development tools
install-tools:
	@echo "Installing development tools..."
	@if ! command -v docker >/dev/null 2>&1; then \
		echo "⚠️  Docker not found. Install from: https://www.docker.com/products/docker-desktop"; \
	else \
		echo "✓ Docker installed: $$(docker --version)"; \
	fi; \
	if ! command -v direnv >/dev/null 2>&1; then \
		echo "Installing direnv..."; \
		if command -v brew >/dev/null 2>&1; then \
			brew install direnv || echo "⚠️  Failed to install direnv. Install manually: brew install direnv"; \
		else \
			echo "⚠️  Homebrew not found. Install direnv manually: brew install direnv"; \
		fi; \
		if [ -f "$$HOME/.zshrc" ] && ! grep -q "direnv hook zsh" "$$HOME/.zshrc"; then \
			echo "Adding direnv hook to .zshrc..."; \
			echo 'eval "$$(direnv hook zsh)"' >> "$$HOME/.zshrc"; \
			echo "✓ direnv hook added to .zshrc (restart your shell or run: source ~/.zshrc)"; \
		elif [ -f "$$HOME/.bashrc" ] && ! grep -q "direnv hook bash" "$$HOME/.bashrc"; then \
			echo "Adding direnv hook to .bashrc..."; \
			echo 'eval "$$(direnv hook bash)"' >> "$$HOME/.bashrc"; \
			echo "✓ direnv hook added to .bashrc (restart your shell or run: source ~/.bashrc)"; \
		fi; \
	else \
		echo "✓ direnv installed: $$(direnv --version)"; \
	fi; \
	if ! command -v op >/dev/null 2>&1; then \
		echo "Installing 1Password CLI..."; \
		if command -v brew >/dev/null 2>&1; then \
			brew install --cask 1password-cli || echo "⚠️  Failed to install 1Password CLI. Install manually: brew install --cask 1password-cli"; \
		else \
			echo "⚠️  Homebrew not found. Install 1Password CLI manually: brew install --cask 1password-cli"; \
		fi; \
	else \
		echo "✓ 1Password CLI installed: $$(op --version)"; \
	fi; \
	if ! command -v fastlane >/dev/null 2>&1 && [ -d "fastlane" ]; then \
		echo "Installing Fastlane..."; \
		if command -v gem >/dev/null 2>&1; then \
			gem install fastlane || echo "⚠️  Failed to install Fastlane"; \
		else \
			echo "⚠️  Ruby/gem not found. Install Fastlane manually: gem install fastlane"; \
		fi; \
	else \
		echo "✓ Fastlane installed (or not needed)"; \
	fi; \
	echo "✓ Development tools check complete"
