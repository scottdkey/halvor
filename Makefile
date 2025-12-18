
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
	@OS=$$(uname -s); \
	if [ "$$OS" = "Darwin" ]; then \
		echo "You can now run: halvor build cli, halvor build ios, halvor build mac, halvor build android, or halvor build web"; \
	else \
		echo "You can now run: halvor build cli or halvor build web"; \
	fi

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
	OS=$$(uname -s); \
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
	if [ "$$OS" = "Linux" ]; then \
		echo "Installing Linux cross-compilation toolchains..."; \
		if command -v apt-get >/dev/null 2>&1; then \
			sudo apt-get install -y gcc-aarch64-linux-gnu gcc-x86-64-linux-gnu musl-tools || echo "⚠️  Failed to install cross-compilation toolchains"; \
		else \
			echo "ℹ️  Cross-compilation toolchains must be installed manually on non-Debian systems"; \
		fi; \
	fi; \
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
	@OS=$$(uname -s); \
	if [ "$$OS" = "Darwin" ]; then \
		if ! command -v swift >/dev/null 2>&1; then \
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
	else \
		echo "ℹ️  Swift/Xcode only available on macOS (skipping)"; \
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
	@OS=$$(uname -s); \
	if [ -z "$$ANDROID_NDK_HOME" ] && [ -z "$$ANDROID_NDK_ROOT" ]; then \
		if [ "$$OS" = "Darwin" ]; then \
			echo "⚠️  Android NDK not found. Please set ANDROID_NDK_HOME or ANDROID_NDK_ROOT"; \
			echo "   Install via Android Studio SDK Manager or download from:"; \
			echo "   https://developer.android.com/ndk/downloads"; \
		else \
			echo "ℹ️  Android NDK not configured (optional on Linux)"; \
		fi; \
	else \
		echo "✓ Android NDK found: $$ANDROID_NDK_HOME$$ANDROID_NDK_ROOT"; \
	fi; \
	if ! command -v java >/dev/null 2>&1; then \
		if [ "$$OS" = "Darwin" ]; then \
			echo "⚠️  Java not found. Android builds require Java 17+"; \
			echo "   Install via: brew install openjdk@17"; \
		elif [ "$$OS" = "Linux" ]; then \
			echo "Java not found. Installing OpenJDK 17..."; \
			if command -v apt-get >/dev/null 2>&1; then \
				sudo apt-get update && sudo apt-get install -y openjdk-17-jdk || echo "⚠️  Failed to install Java via apt"; \
			elif command -v yum >/dev/null 2>&1; then \
				sudo yum install -y java-17-openjdk-devel || echo "⚠️  Failed to install Java via yum"; \
			elif command -v dnf >/dev/null 2>&1; then \
				sudo dnf install -y java-17-openjdk-devel || echo "⚠️  Failed to install Java via dnf"; \
			elif command -v pacman >/dev/null 2>&1; then \
				sudo pacman -S --noconfirm jdk17-openjdk || echo "⚠️  Failed to install Java via pacman"; \
			else \
				echo "⚠️  No supported package manager found. Install Java manually"; \
			fi; \
		fi; \
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
	@OS=$$(uname -s); \
	export NVM_DIR="$$HOME/.nvm"; \
	if [ ! -d "$$NVM_DIR" ]; then \
		echo "Installing NVM (Node Version Manager)..."; \
		curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.1/install.sh | bash || echo "⚠️  Failed to install NVM"; \
	fi; \
	if [ -s "$$NVM_DIR/nvm.sh" ]; then \
		. "$$NVM_DIR/nvm.sh"; \
		if ! command -v node >/dev/null 2>&1 || [ "$$(node --version | cut -d'.' -f1 | tr -d 'v')" -lt 24 ]; then \
			echo "Installing Node.js 24 LTS via NVM..."; \
			nvm install 24 || echo "⚠️  Failed to install Node.js 24 via NVM"; \
			nvm use 24 || true; \
			nvm alias default 24 || true; \
		fi; \
		if command -v node >/dev/null 2>&1; then \
			echo "✓ Node.js installed: $$(node --version)"; \
		else \
			echo "⚠️  Node.js installation failed. Try manually: nvm install 24"; \
		fi; \
		if command -v npm >/dev/null 2>&1; then \
			echo "✓ npm installed: $$(npm --version)"; \
		else \
			echo "⚠️  npm not found. This should come with Node.js."; \
		fi; \
	else \
		echo "⚠️  NVM installation failed. Install manually from: https://github.com/nvm-sh/nvm"; \
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
	@export NVM_DIR="$$HOME/.nvm"; \
	if [ -s "$$NVM_DIR/nvm.sh" ]; then \
		. "$$NVM_DIR/nvm.sh"; \
	fi; \
	if [ -d "halvor-web" ]; then \
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
	@# Detect OS
	@OS=$$(uname -s); \
	if ! command -v docker >/dev/null 2>&1; then \
		echo "⚠️  Docker not found. Install from: https://www.docker.com/products/docker-desktop"; \
	else \
		echo "✓ Docker installed: $$(docker --version)"; \
	fi; \
	\
	if ! command -v direnv >/dev/null 2>&1; then \
		echo "Installing direnv..."; \
		if [ "$$OS" = "Darwin" ]; then \
			if command -v brew >/dev/null 2>&1; then \
				brew install direnv || echo "⚠️  Failed to install direnv"; \
			else \
				echo "⚠️  Homebrew not found. Install direnv manually: brew install direnv"; \
			fi; \
		elif [ "$$OS" = "Linux" ]; then \
			if command -v apt-get >/dev/null 2>&1; then \
				echo "Detected Debian/Ubuntu, installing direnv via apt..."; \
				sudo apt-get update && sudo apt-get install -y direnv || echo "⚠️  Failed to install direnv via apt"; \
			elif command -v yum >/dev/null 2>&1; then \
				echo "Detected RHEL/CentOS/Fedora, installing direnv via yum..."; \
				sudo yum install -y direnv || echo "⚠️  Failed to install direnv via yum"; \
			elif command -v dnf >/dev/null 2>&1; then \
				echo "Detected Fedora/RHEL 8+, installing direnv via dnf..."; \
				sudo dnf install -y direnv || echo "⚠️  Failed to install direnv via dnf"; \
			elif command -v pacman >/dev/null 2>&1; then \
				echo "Detected Arch Linux, installing direnv via pacman..."; \
				sudo pacman -S --noconfirm direnv || echo "⚠️  Failed to install direnv via pacman"; \
			elif command -v zypper >/dev/null 2>&1; then \
				echo "Detected openSUSE, installing direnv via zypper..."; \
				sudo zypper install -y direnv || echo "⚠️  Failed to install direnv via zypper"; \
			else \
				echo "⚠️  No supported package manager found. Install direnv manually from: https://direnv.net/docs/installation.html"; \
			fi; \
		else \
			echo "⚠️  Unsupported OS: $$OS. Install direnv manually from: https://direnv.net/docs/installation.html"; \
		fi; \
	else \
		echo "✓ direnv installed: $$(direnv --version)"; \
	fi; \
	\
	if command -v direnv >/dev/null 2>&1; then \
		SHELL_NAME=$$(basename "$$SHELL"); \
		if [ "$$SHELL_NAME" = "zsh" ]; then \
			if [ -f "$$HOME/.zshrc" ] && ! grep -q "direnv hook zsh" "$$HOME/.zshrc"; then \
				echo "Adding direnv hook to .zshrc..."; \
				echo 'eval "$$(direnv hook zsh)"' >> "$$HOME/.zshrc"; \
				echo "✓ direnv hook added to .zshrc (restart your shell or run: source ~/.zshrc)"; \
			fi; \
		elif [ "$$SHELL_NAME" = "bash" ]; then \
			if [ -f "$$HOME/.bashrc" ] && ! grep -q "direnv hook bash" "$$HOME/.bashrc"; then \
				echo "Adding direnv hook to .bashrc..."; \
				echo 'eval "$$(direnv hook bash)"' >> "$$HOME/.bashrc"; \
				echo "✓ direnv hook added to .bashrc (restart your shell or run: source ~/.bashrc)"; \
			fi; \
		fi; \
	fi; \
	\
	if ! command -v op >/dev/null 2>&1; then \
		echo "Installing 1Password CLI..."; \
		if [ "$$OS" = "Darwin" ]; then \
			if command -v brew >/dev/null 2>&1; then \
				brew install --cask 1password-cli || echo "⚠️  Failed to install 1Password CLI"; \
			else \
				echo "⚠️  Homebrew not found. Install 1Password CLI manually: brew install --cask 1password-cli"; \
			fi; \
		elif [ "$$OS" = "Linux" ]; then \
			echo "Installing 1Password CLI for Linux..."; \
			curl -sS https://downloads.1password.com/linux/keys/1password.asc | sudo gpg --dearmor --output /usr/share/keyrings/1password-archive-keyring.gpg && \
			echo "deb [arch=$$(dpkg --print-architecture) signed-by=/usr/share/keyrings/1password-archive-keyring.gpg] https://downloads.1password.com/linux/debian/$$( . /etc/os-release && echo "$$VERSION_CODENAME" ) stable" | sudo tee /etc/apt/sources.list.d/1password.list && \
			sudo mkdir -p /etc/debsig/policies/AC2D62742012EA22/ && \
			curl -sS https://downloads.1password.com/linux/debian/debsig/1password.pol | sudo tee /etc/debsig/policies/AC2D62742012EA22/1password.pol && \
			sudo mkdir -p /usr/share/debsig/keyrings/AC2D62742012EA22 && \
			curl -sS https://downloads.1password.com/linux/keys/1password.asc | sudo gpg --dearmor --output /usr/share/debsig/keyrings/AC2D62742012EA22/debsig.gpg && \
			sudo apt-get update && sudo apt-get install -y 1password-cli || echo "⚠️  Failed to install 1Password CLI. See: https://developer.1password.com/docs/cli/get-started/#install"; \
		else \
			echo "⚠️  Unsupported OS for 1Password CLI auto-install. See: https://developer.1password.com/docs/cli/get-started/#install"; \
		fi; \
	else \
		echo "✓ 1Password CLI installed: $$(op --version)"; \
	fi; \
	\
	if [ "$$OS" = "Darwin" ]; then \
		if ! command -v ruby >/dev/null 2>&1; then \
			echo "Installing Ruby..."; \
			if command -v brew >/dev/null 2>&1; then \
				brew install ruby || echo "⚠️  Failed to install Ruby via Homebrew"; \
			else \
				echo "⚠️  Homebrew not found. Install Ruby manually: brew install ruby"; \
			fi; \
		else \
			echo "✓ Ruby installed: $$(ruby --version)"; \
		fi; \
		\
		if ! command -v gem >/dev/null 2>&1; then \
			echo "⚠️  RubyGems (gem) not found. This should come with Ruby. Please reinstall Ruby."; \
		else \
			echo "✓ RubyGems installed: $$(gem --version)"; \
		fi; \
		\
		if command -v gem >/dev/null 2>&1; then \
			if ! command -v fastlane >/dev/null 2>&1; then \
				echo "Installing Fastlane via RubyGems..."; \
				echo "This may take a few minutes on first install..."; \
				gem install fastlane --user-install --no-document || echo "⚠️  Failed to install Fastlane via user install. Fastlane is optional for iOS/macOS builds."; \
			else \
				echo "✓ Fastlane installed: $$(fastlane --version | head -1)"; \
			fi; \
			if [ -d "fastlane" ]; then \
				echo "Installing Fastlane dependencies from Gemfile..."; \
				cd fastlane && (bundle install --quiet || gem install bundler && bundle install --quiet) 2>/dev/null || echo "⚠️  Failed to install Fastlane bundle dependencies"; \
			fi; \
		fi; \
	else \
		echo "ℹ️  Ruby/Fastlane only needed on macOS for iOS/macOS builds (skipping)"; \
	fi; \
	echo "✓ Development tools installation complete"
