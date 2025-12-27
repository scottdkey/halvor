
# Installation and setup
.PHONY: install install-rust install-rust-targets install-rust-deps install-swift install-swift-deps install-android install-android-deps install-web install-web-deps install-tools help docs

# Default target
help:
	@echo "Halvor Multi-Platform Build System"
	@echo ""
	@echo "Installation:"
	@echo "  make install              - Install all dependencies (Rust, Swift, Android, Web)"
	@echo "  make install-rust         - Install Rust toolchain"
	@echo "  make install-rust-targets - Install all Rust targets"
	@echo "  make install-swift        - Install Swift/Xcode dependencies"
	@echo "  make install-android      - Install Android dependencies"
	@echo "  make install-web          - Install Web dependencies (Node.js, wasm-pack)"
	@echo "  make install-tools         - Install development tools (Docker, Fastlane)"
	@echo "  make install-cli          - Build and install CLI to system"
	@echo "  make install-agent        - Build and install agent binary to system"
	@echo ""
	@echo "Build targets (use 'halvor build <subcommand>' or 'make build-<target>'):"
	@echo "  make build-cli            - Build CLI binary (uses halvor-build)"
	@echo "  make build-agent          - Build agent binary"
	@echo "  make build-ios            - Build iOS Swift app (uses halvor-build)"
	@echo "  make build-mac            - Build macOS Swift app (uses halvor-build)"
	@echo "  make build-android        - Build Android library and app (uses halvor-build)"
	@echo "  make build-web            - Build WASM module and web app (uses halvor-build)"
	@echo "  make build-docker-pia-vpn - Build PIA VPN Docker container (uses halvor-build)"
	@echo "  make build-docker-agent   - Build agent server Docker container"
	@echo "  make build-helm-<chart>   - Build/package Helm chart (e.g., make build-helm-portainer)"
	@echo "  make build-all            - Build all CLIs, containers, and Helm charts"
	@echo ""
	@echo "Development (use 'make dev <subcommand>'):"
	@echo "  make dev mac               - macOS development with hot reload"
	@echo "  make dev ios               - iOS development with simulator"
	@echo "  make dev web               - Web development with hot reload (Docker)"
	@echo "  make dev web-bare-metal    - Web development (Rust server + Svelte dev)"
	@echo "  make dev web-prod          - Web production mode (Docker)"
	@echo "  make dev cli               - CLI development mode with watch (auto-rebuild on changes)"
	@echo ""
	@echo "Documentation:"
	@echo "  make docs                 - Generate documentation (CLI commands, Docker containers, Helm charts)"


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

# Install CLI to system (stops agent service, builds, installs, restarts service)
.PHONY: install-cli
install-cli:
	@echo "Building and installing CLI to system..."
	@# Stop agent service if running (macOS)
	@if [ "$$(uname -s)" = "Darwin" ]; then \
		if launchctl list com.halvor.agent 2>/dev/null | grep -q .; then \
			echo "Stopping halvor agent service..."; \
			launchctl stop com.halvor.agent 2>/dev/null || true; \
			launchctl unload ~/Library/LaunchAgents/com.halvor.agent.plist 2>/dev/null || true; \
			sleep 1; \
		fi; \
		pkill -9 -f "halvor agent" 2>/dev/null || true; \
	fi
	@# Stop agent service if running (Linux)
	@if [ "$$(uname -s)" = "Linux" ]; then \
		if systemctl is-active halvor-agent.service >/dev/null 2>&1; then \
			echo "Stopping halvor agent service..."; \
			sudo systemctl stop halvor-agent.service 2>/dev/null || true; \
			sleep 1; \
		fi; \
	fi
	@cargo build --release --bin halvor --manifest-path crates/halvor-cli/Cargo.toml
	@mkdir -p ~/.cargo/bin
	@cp -f target/release/halvor ~/.cargo/bin/halvor
	@echo "CLI installed to ~/.cargo/bin/halvor (available as 'halvor')"
	@# Restart agent service if plist/service file exists (macOS)
	@if [ "$$(uname -s)" = "Darwin" ]; then \
		if [ -f ~/Library/LaunchAgents/com.halvor.agent.plist ]; then \
			echo "Restarting halvor agent service..."; \
			launchctl load -w ~/Library/LaunchAgents/com.halvor.agent.plist 2>/dev/null || true; \
			launchctl start com.halvor.agent 2>/dev/null || true; \
			echo "Agent service restarted"; \
		fi; \
	fi
	@# Restart agent service if service file exists (Linux)
	@if [ "$$(uname -s)" = "Linux" ]; then \
		# Try user service first (no sudo needed)
		if [ -f ~/.config/systemd/user/halvor-agent.service ]; then \
			echo "Restarting halvor agent service (user service)..."; \
			systemctl --user daemon-reload 2>/dev/null || true; \
			systemctl --user restart halvor-agent.service 2>/dev/null || true; \
			echo "Agent service restarted"; \
		# Fall back to system service (requires sudo)
		elif [ -f /etc/systemd/system/halvor-agent.service ]; then \
			echo "Restarting halvor agent service (system service, requires sudo)..."; \
			sudo systemctl daemon-reload; \
			sudo systemctl start halvor-agent.service 2>/dev/null || true; \
			echo "Agent service restarted"; \
			echo ""; \
			echo "Note: Service is installed as a system service, which requires sudo."; \
			echo "   To avoid sudo, reinstall the service: halvor agent start --daemon"; \
		fi; \
	fi

# Install agent binary to system (agent only, no CLI)
.PHONY: install-agent
install-agent:
	@echo "Building and installing agent binary to system..."
	@cargo build --release --bin halvor-agent --manifest-path crates/halvor-agent/Cargo.toml
	@mkdir -p ~/.cargo/bin
	@cp -f target/release/halvor-agent ~/.cargo/bin/halvor-agent
	@echo "✓ Agent installed to ~/.cargo/bin/halvor-agent (available as 'halvor-agent')"
	@echo "  Start with: halvor-agent --port 13500"
	@echo "  Or use CLI: halvor agent start --port 13500 --daemon"
	@echo "  For web UI: halvor agent start --port 13500 --ui --daemon"

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
	echo "Installing Linux targets..."; \
	rustup target add x86_64-unknown-linux-gnu || true; \
	rustup target add aarch64-unknown-linux-gnu || true; \
	rustup target add x86_64-unknown-linux-musl || true; \
	rustup target add aarch64-unknown-linux-musl || true; \
	if [ "$$OS" = "Linux" ]; then \
		echo "Installing Linux toolchains..."; \
		if command -v apt-get >/dev/null 2>&1; then \
			sudo apt-get install -y gcc-aarch64-linux-gnu gcc-x86-64-linux-gnu musl-tools || echo "⚠️  Failed to install toolchains"; \
		else \
			echo "ℹ️  Toolchains must be installed manually on non-Debian systems"; \
		fi; \
	fi; \
	echo "Installing Windows targets..."; \
	rustup target add x86_64-pc-windows-msvc || true; \
	rustup target add aarch64-pc-windows-msvc || true; \
	echo "Installing WASM target..."; \
	rustup target add wasm32-unknown-unknown || true; \
	echo "✓ All Rust targets installed"

# Install Rust crate dependencies
install-rust-deps: install-rust
	@echo "Installing Rust crate dependencies..."
	@. $$HOME/.cargo/env 2>/dev/null || true; \
	OS=$$(uname -s); \
	echo "Checking for C compiler (cc/gcc)..."; \
	if ! command -v cc >/dev/null 2>&1 && ! command -v gcc >/dev/null 2>&1; then \
		echo "C compiler (cc) not found. Installing for platform..."; \
		if [ "$$OS" = "Linux" ]; then \
			if command -v apt-get >/dev/null 2>&1; then \
				echo "Installing build-essential (includes gcc/cc) via apt..."; \
				if [ -f /etc/apt/sources.list.d/1password.list ]; then \
					if ! sudo apt-get update 2>&1 | grep -q "Malformed entry"; then \
						:; \
					else \
						echo "⚠️  Detected malformed repository file. Cleaning up..."; \
						sudo rm -f /etc/apt/sources.list.d/1password.list; \
					fi; \
				fi; \
				sudo apt-get update && sudo apt-get install -y build-essential || echo "⚠️  Failed to install build-essential"; \
			elif command -v yum >/dev/null 2>&1; then \
				echo "Installing gcc via yum..."; \
				sudo yum install -y gcc || echo "⚠️  Failed to install gcc via yum"; \
			elif command -v dnf >/dev/null 2>&1; then \
				echo "Installing gcc via dnf..."; \
				sudo dnf install -y gcc || echo "⚠️  Failed to install gcc via dnf"; \
			elif command -v pacman >/dev/null 2>&1; then \
				echo "Installing base-devel (includes gcc) via pacman..."; \
				sudo pacman -S --noconfirm base-devel || echo "⚠️  Failed to install base-devel"; \
			elif command -v zypper >/dev/null 2>&1; then \
				echo "Installing gcc via zypper..."; \
				sudo zypper install -y gcc || echo "⚠️  Failed to install gcc via zypper"; \
			else \
				echo "⚠️  No supported package manager found. Please install gcc/cc manually"; \
			fi; \
		elif [ "$$OS" = "Darwin" ]; then \
			if command -v brew >/dev/null 2>&1; then \
				echo "Installing Xcode Command Line Tools (includes cc)..."; \
				xcode-select --install 2>/dev/null || echo "⚠️  Xcode Command Line Tools may already be installed or installation requires manual approval"; \
			else \
				echo "⚠️  Please install Xcode Command Line Tools manually: xcode-select --install"; \
			fi; \
		else \
			echo "⚠️  Unsupported OS: $$OS. Please install gcc/cc manually"; \
		fi; \
		if command -v cc >/dev/null 2>&1 || command -v gcc >/dev/null 2>&1; then \
			echo "✓ C compiler installed: $$(cc --version 2>&1 | head -1 || gcc --version 2>&1 | head -1)"; \
		else \
			echo "⚠️  C compiler installation may have failed. Continuing anyway..."; \
		fi; \
	else \
		echo "✓ C compiler found: $$(cc --version 2>&1 | head -1 || gcc --version 2>&1 | head -1)"; \
	fi; \
	echo "Checking for 'cross' tool (not needed, will remove if found)..."; \
	if command -v cross >/dev/null 2>&1; then \
		echo "  ⚠️  Found 'cross' tool installed. Removing it (not needed for this project)..."; \
		cargo uninstall cross 2>/dev/null || echo "  ⚠️  Failed to uninstall cross via cargo. You may need to remove it manually: cargo uninstall cross"; \
		if command -v cross >/dev/null 2>&1; then \
			echo "  ⚠️  'cross' is still installed. It may have been installed via a package manager."; \
			echo "      This project does not use 'cross' - you can safely ignore this or remove it manually."; \
		else \
			echo "  ✓ 'cross' removed successfully"; \
		fi; \
	else \
		echo "  ✓ 'cross' not installed (as expected)"; \
	fi; \
	echo "Fetching dependencies for main crate..."; \
	cargo fetch || echo "⚠️  Failed to fetch main crate dependencies"; \
	if [ -d "projects/ios/halvor-ffi" ]; then \
		echo "Fetching dependencies for halvor-ffi..."; \
		cd projects/ios/halvor-ffi && cargo fetch || echo "⚠️  Failed to fetch halvor-ffi dependencies"; \
	fi; \
	if [ -d "projects/ios/halvor-ffi-macro" ]; then \
		echo "Fetching dependencies for halvor-ffi-macro..."; \
		cd projects/ios/halvor-ffi-macro && cargo fetch || echo "⚠️  Failed to fetch halvor-ffi-macro dependencies"; \
	fi; \
	if [ -d "projects/ios/halvor-ffi-jni" ]; then \
		echo "Fetching dependencies for halvor-ffi-jni..."; \
		cd projects/ios/halvor-ffi-jni && cargo fetch || echo "⚠️  Failed to fetch halvor-ffi-jni dependencies"; \
	fi; \
	if [ -d "projects/ios/halvor-ffi-wasm" ]; then \
		echo "Fetching dependencies for halvor-ffi-wasm..."; \
		cd projects/ios/halvor-ffi-wasm && cargo fetch || echo "⚠️  Failed to fetch halvor-ffi-wasm dependencies"; \
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
	elif [ "$$OS" = "Linux" ]; then \
		if ! command -v swift >/dev/null 2>&1; then \
			echo "⚠️  Swift not found on Linux."; \
			if command -v apt-get >/dev/null 2>&1; then \
				echo "   Attempting to install Swift via package manager..."; \
				if sudo apt-get update && sudo apt-get install -y swift 2>/dev/null; then \
					echo "✓ Swift installed via package manager"; \
				else \
					echo "   Swift not available via package manager."; \
					echo "   Install manually from: https://www.swift.org/download/"; \
					echo "   Or download and extract Swift to /opt/swift and add to PATH"; \
				fi; \
			else \
				echo "   Install Swift manually from: https://www.swift.org/download/"; \
			fi; \
		else \
			echo "✓ Swift installed: $$(swift --version | head -1)"; \
		fi; \
	else \
		echo "ℹ️  Swift/Xcode only available on macOS/Linux (skipping on other platforms)"; \
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
	@if [ -d "projects/ios" ]; then \
		cd projects/ios && \
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
			echo "Java not found. Installing OpenJDK 17 (required for Android)..."; \
			if command -v apt-get >/dev/null 2>&1; then \
				sudo apt-get update && sudo apt-get install -y openjdk-17-jdk || echo "⚠️  Failed to install openjdk-17-jdk. Install manually: sudo apt-get install -y openjdk-17-jdk"; \
			elif command -v yum >/dev/null 2>&1; then \
				sudo yum install -y java-17-openjdk-devel || echo "⚠️  Failed to install Java via yum"; \
			elif command -v dnf >/dev/null 2>&1; then \
				sudo dnf install -y java-17-openjdk-devel || echo "⚠️  Failed to install Java via dnf"; \
			elif command -v pacman >/dev/null 2>&1; then \
				sudo pacman -S --noconfirm jdk17-openjdk || echo "⚠️  Failed to install Java via pacman"; \
			else \
				echo "⚠️  No supported package manager found. Install Java 17 manually"; \
			fi; \
		fi; \
	else \
		echo "✓ Java installed: $$(java -version 2>&1 | head -1)"; \
	fi; \
	if [ -d "projects/android" ]; then \
		echo "Checking Gradle wrapper..."; \
		cd projects/android && chmod +x gradlew 2>/dev/null || true; \
	fi

# Install Android Gradle dependencies
install-android-deps: install-android
	@echo "Installing Android Gradle dependencies..."
	@if [ -d "projects/android" ]; then \
		cd projects/android && \
		if [ -f "gradlew" ]; then \
			echo "Downloading Gradle and dependencies..."; \
			chmod +x gradlew && \
			./gradlew --no-daemon tasks --all >/dev/null 2>&1 || ./gradlew --no-daemon build --dry-run || echo "⚠️  Failed to download Gradle dependencies"; \
			echo "✓ Android Gradle dependencies installed"; \
		else \
			echo "Gradle wrapper not found. Initializing Gradle wrapper..."; \
			if command -v gradle >/dev/null 2>&1; then \
				gradle wrapper --gradle-version 8.5 || echo "⚠️  Failed to initialize Gradle wrapper. Install Gradle: https://gradle.org/install/"; \
			else \
				echo "⚠️  Gradle not found. Install Gradle to initialize wrapper:"; \
				echo "   https://gradle.org/install/"; \
				echo "   Or run: gradle wrapper --gradle-version 8.5"; \
			fi; \
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
	if [ -d "projects/web" ]; then \
		cd projects/web && \
		if command -v npm >/dev/null 2>&1; then \
			echo "Running npm install..."; \
			npm install || echo "⚠️  Failed to install npm dependencies"; \
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
	if ! command -v cc >/dev/null 2>&1 && ! command -v gcc >/dev/null 2>&1; then \
		echo "Installing C compiler (build-essential)..."; \
		if [ "$$OS" = "Linux" ]; then \
			if command -v apt-get >/dev/null 2>&1; then \
				if [ -f /etc/apt/sources.list.d/1password.list ]; then \
					if ! sudo apt-get update 2>&1 | grep -q "Malformed entry"; then \
						:; \
					else \
						echo "⚠️  Detected malformed repository file. Cleaning up..."; \
						sudo rm -f /etc/apt/sources.list.d/1password.list; \
					fi; \
				fi; \
				sudo apt-get update && sudo apt-get install -y build-essential || echo "⚠️  Failed to install build-essential"; \
			elif command -v yum >/dev/null 2>&1; then \
				sudo yum install -y gcc || echo "⚠️  Failed to install gcc via yum"; \
			elif command -v dnf >/dev/null 2>&1; then \
				sudo dnf install -y gcc || echo "⚠️  Failed to install gcc via dnf"; \
			elif command -v pacman >/dev/null 2>&1; then \
				sudo pacman -S --noconfirm base-devel || echo "⚠️  Failed to install base-devel"; \
			elif command -v zypper >/dev/null 2>&1; then \
				sudo zypper install -y gcc || echo "⚠️  Failed to install gcc via zypper"; \
			else \
				echo "⚠️  No supported package manager found. Please install gcc/cc manually"; \
			fi; \
		elif [ "$$OS" = "Darwin" ]; then \
			if command -v brew >/dev/null 2>&1; then \
				echo "Installing Xcode Command Line Tools (includes cc)..."; \
				xcode-select --install 2>/dev/null || echo "⚠️  Xcode Command Line Tools may already be installed or installation requires manual approval"; \
			else \
				echo "⚠️  Please install Xcode Command Line Tools manually: xcode-select --install"; \
			fi; \
		fi; \
		if command -v cc >/dev/null 2>&1 || command -v gcc >/dev/null 2>&1; then \
			echo "✓ C compiler installed: $$(cc --version 2>&1 | head -1 || gcc --version 2>&1 | head -1)"; \
		fi; \
	else \
		echo "✓ C compiler found: $$(cc --version 2>&1 | head -1 || gcc --version 2>&1 | head -1)"; \
	fi; \
	if ! command -v docker >/dev/null 2>&1; then \
		echo "Installing Docker..."; \
		if [ "$$OS" = "Darwin" ]; then \
			if command -v brew >/dev/null 2>&1; then \
				brew install --cask docker || echo "⚠️  Failed to install Docker via Homebrew. Install from: https://www.docker.com/products/docker-desktop"; \
			else \
				echo "⚠️  Homebrew not found. Install Docker from: https://www.docker.com/products/docker-desktop"; \
			fi; \
		elif [ "$$OS" = "Linux" ]; then \
			if command -v apt-get >/dev/null 2>&1; then \
				echo "Installing Docker via apt..."; \
				if ! command -v curl >/dev/null 2>&1; then \
					echo "Installing curl first..."; \
					sudo apt-get update && sudo apt-get install -y curl ca-certificates gnupg || echo "⚠️  Failed to install prerequisites"; \
				fi; \
				if command -v curl >/dev/null 2>&1; then \
					sudo install -m 0755 -d /etc/apt/keyrings && \
					curl -fsSL https://download.docker.com/linux/debian/gpg | sudo gpg --dearmor -o /etc/apt/keyrings/docker.gpg && \
					sudo chmod a+r /etc/apt/keyrings/docker.gpg && \
					echo "deb [arch=$$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/debian $$(. /etc/os-release && echo "$$VERSION_CODENAME") stable" | sudo tee /etc/apt/sources.list.d/docker.list >/dev/null && \
					sudo apt-get update && sudo apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin || echo "⚠️  Failed to install Docker. See: https://docs.docker.com/engine/install/debian/"; \
				else \
					echo "⚠️  curl not available. Cannot install Docker automatically."; \
				fi; \
			elif command -v yum >/dev/null 2>&1; then \
				echo "Installing Docker via yum..."; \
				sudo yum install -y yum-utils && \
				sudo yum-config-manager --add-repo https://download.docker.com/linux/centos/docker-ce.repo && \
				sudo yum install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin || echo "⚠️  Failed to install Docker via yum"; \
			elif command -v dnf >/dev/null 2>&1; then \
				echo "Installing Docker via dnf..."; \
				sudo dnf install -y dnf-plugins-core && \
				sudo dnf config-manager --add-repo https://download.docker.com/linux/fedora/docker-ce.repo && \
				sudo dnf install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin || echo "⚠️  Failed to install Docker via dnf"; \
			else \
				echo "⚠️  Unsupported package manager. Install Docker from: https://docs.docker.com/engine/install/"; \
			fi; \
			if command -v docker >/dev/null 2>&1; then \
				echo "✓ Docker installed: $$(docker --version)"; \
				if command -v systemctl >/dev/null 2>&1; then \
					echo "Starting Docker service..."; \
					sudo systemctl enable docker && sudo systemctl start docker || echo "⚠️  Failed to start Docker service"; \
				fi; \
			fi; \
		else \
			echo "⚠️  Unsupported OS: $$OS. Install Docker from: https://www.docker.com/products/docker-desktop"; \
		fi; \
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
	if ! command -v jq >/dev/null 2>&1; then \
		echo "Installing jq (required for 1Password CLI integration)..."; \
		if [ "$$OS" = "Darwin" ]; then \
			if command -v brew >/dev/null 2>&1; then \
				brew install jq || echo "⚠️  Failed to install jq"; \
			else \
				echo "⚠️  Homebrew not found. Install jq manually: brew install jq"; \
			fi; \
		elif [ "$$OS" = "Linux" ]; then \
			if command -v apt-get >/dev/null 2>&1; then \
				echo "Installing jq via apt..."; \
				sudo apt-get update && sudo apt-get install -y jq || echo "⚠️  Failed to install jq via apt"; \
			elif command -v yum >/dev/null 2>&1; then \
				echo "Installing jq via yum..."; \
				sudo yum install -y jq || echo "⚠️  Failed to install jq via yum"; \
			elif command -v dnf >/dev/null 2>&1; then \
				echo "Installing jq via dnf..."; \
				sudo dnf install -y jq || echo "⚠️  Failed to install jq via dnf"; \
			elif command -v pacman >/dev/null 2>&1; then \
				echo "Installing jq via pacman..."; \
				sudo pacman -S --noconfirm jq || echo "⚠️  Failed to install jq via pacman"; \
			elif command -v zypper >/dev/null 2>&1; then \
				echo "Installing jq via zypper..."; \
				sudo zypper install -y jq || echo "⚠️  Failed to install jq via zypper"; \
			else \
				echo "⚠️  No supported package manager found. Install jq manually from: https://stedolan.github.io/jq/download/"; \
			fi; \
		else \
			echo "⚠️  Unsupported OS: $$OS. Install jq manually from: https://stedolan.github.io/jq/download/"; \
		fi; \
		if command -v jq >/dev/null 2>&1; then \
			echo "✓ jq installed: $$(jq --version)"; \
		fi; \
	else \
		echo "✓ jq installed: $$(jq --version)"; \
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
			if [ -f /etc/apt/sources.list.d/1password.list ]; then \
				echo "Removing existing 1Password repository file (will recreate)..."; \
				sudo rm -f /etc/apt/sources.list.d/1password.list; \
			fi; \
			if ! command -v apt-get >/dev/null 2>&1; then \
				echo "⚠️  apt-get not found. 1Password CLI installation requires Debian/Ubuntu with apt-get. See: https://developer.1password.com/docs/cli/get-started/#install"; \
			elif ! command -v curl >/dev/null 2>&1; then \
				echo "⚠️  curl not found. Installing curl first..."; \
				sudo apt-get update && sudo apt-get install -y curl || echo "⚠️  Failed to install curl. Please install curl manually."; \
			elif ! command -v gpg >/dev/null 2>&1; then \
				echo "⚠️  gpg not found. Installing gpg first..."; \
				sudo apt-get update && sudo apt-get install -y gpg || echo "⚠️  Failed to install gpg. Please install gpg manually."; \
			else \
				curl -sS https://downloads.1password.com/linux/keys/1password.asc | \
				  sudo gpg --dearmor --output /usr/share/keyrings/1password-archive-keyring.gpg && \
				  echo "deb [arch=$$(dpkg --print-architecture) signed-by=/usr/share/keyrings/1password-archive-keyring.gpg] https://downloads.1password.com/linux/debian/$$(dpkg --print-architecture) stable main" | \
				  sudo tee /etc/apt/sources.list.d/1password.list && \
				  sudo mkdir -p /etc/debsig/policies/AC2D62742012EA22/ && \
				  curl -sS https://downloads.1password.com/linux/debian/debsig/1password.pol | \
				  sudo tee /etc/debsig/policies/AC2D62742012EA22/1password.pol && \
				  sudo mkdir -p /usr/share/debsig/keyrings/AC2D62742012EA22 && \
				  curl -sS https://downloads.1password.com/linux/keys/1password.asc | \
				  sudo gpg --dearmor --output /usr/share/debsig/keyrings/AC2D62742012EA22/debsig.gpg && \
				  sudo apt update && sudo apt install -y 1password-cli || echo "⚠️  Failed to install 1Password CLI. See: https://developer.1password.com/docs/cli/get-started/#install"; \
			fi; \
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

# Build targets using halvor-build crate
.PHONY: build-cli build-agent build-ios build-mac build-android build-web build-docker-pia-vpn build-docker-agent build-helm build-all

build-cli:
	@echo "Building CLI using halvor-build..."
	@cargo run --release --bin halvor -- build cli || cargo run --bin halvor -- build cli

build-agent:
	@echo "Building agent binary..."
	@cargo build --release --bin halvor-agent --manifest-path crates/halvor-agent/Cargo.toml

build-ios:
	@echo "Building iOS app using halvor-build..."
	@cargo run --release --bin halvor -- build ios || cargo run --bin halvor -- build ios

build-mac:
	@echo "Building macOS app using halvor-build..."
	@cargo run --release --bin halvor -- build mac || cargo run --bin halvor -- build mac

build-android:
	@echo "Building Android app using halvor-build..."
	@cargo run --release --bin halvor -- build android || cargo run --bin halvor -- build android

build-web:
	@echo "Building web app using halvor-build..."
	@cargo run --release --bin halvor -- build web || cargo run --bin halvor -- build web

build-docker-pia-vpn:
	@echo "Building PIA VPN Docker container using halvor-build..."
	@cargo run --release --bin halvor -- build pia-vpn || cargo run --bin halvor -- build pia-vpn

build-docker-agent:
	@echo "Building agent server Docker container..."
	@if [ ! -f "projects/agent/Dockerfile" ]; then \
		echo "⚠️  Agent Dockerfile not found. Skipping agent container build."; \
	else \
		docker build -t ghcr.io/scottdkey/halvor-agent:experimental -f projects/agent/Dockerfile . || \
		echo "⚠️  Failed to build agent container. Ensure Dockerfile exists."; \
	fi

build-helm-%:
	@echo "Packaging Helm chart: $*"
	@if [ ! -d "charts/$*" ]; then \
		echo "⚠️  Chart directory not found: charts/$*"; \
		exit 1; \
	fi
	@mkdir -p charts/packages
	@helm package charts/$* --destination charts/packages/ || \
		(echo "⚠️  Helm not installed. Install with: brew install helm (macOS) or apt-get install helm (Linux)"; exit 1)

build-all: build-cli build-agent build-docker-pia-vpn
	@echo "Building all Helm charts..."
	@mkdir -p charts/packages
	@for chart in charts/*/; do \
		if [ -f "$$chart/Chart.yaml" ]; then \
			chart_name=$$(basename $$chart); \
			echo "Packaging $$chart_name..."; \
			$(MAKE) build-helm-$$chart_name || true; \
		fi; \
	done
	@echo "✓ All builds complete"

# Generate documentation
docs:
	@echo "Generating documentation..."
	@chmod +x scripts/generate-docs.sh
	@./scripts/generate-docs.sh
	@echo "✓ Documentation generated in docs/generated/"
