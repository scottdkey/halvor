# Multi-Platform FFI System

This document describes the multi-platform FFI system that generates bindings for Swift (iOS/macOS), Kotlin (Android), and TypeScript (Web/WASM).

## Architecture

```
halvor (Rust core)
    ├── halvor-ffi (C FFI for Swift)
    ├── halvor-ffi-wasm (WASM for Web)
    ├── halvor-ffi-jni (JNI for Android)
    └── halvor-ffi-macro (Code generation macros)
```

## Macros

### `#[swift_export]`
Marks a function for Swift export. Generates C FFI wrappers and Swift bindings.

### `#[kotlin_export]`
Marks a function for Kotlin/Android export. Generates JNI wrappers and Kotlin bindings.

### `#[wasm_export]`
Marks a function for WASM/Web export. Generates wasm-bindgen wrappers and TypeScript bindings.

### `#[multi_platform_export]`
Marks a function for all platforms. Equivalent to using all three macros.

## Usage Example

```rust
use halvor_ffi_macro::multi_platform_export;

#[multi_platform_export]
pub fn discover_agents(client: &HalvorClient) -> Result<Vec<DiscoveredHost>, String> {
    client.discover_agents()
}
```

This single annotation makes the function available in:
- **Swift**: `try client.discoverAgents()`
- **Kotlin**: `client.discoverAgents()`
- **TypeScript**: `await wasmModule.discoverAgents()`

## Build Process

1. **Rust Compilation**: Functions are compiled with platform-specific targets
2. **Code Generation**: Build scripts generate bindings for each platform
3. **Integration**: Generated code is automatically included in each platform's build

## Platform-Specific Builds

### CLI Binaries

```bash
# Build for current platform (auto-detects OS)
halvor build cli

# Build for specific platforms
halvor build cli --platforms apple
halvor build cli --platforms linux
halvor build cli --platforms windows
halvor build cli --platforms apple,linux,windows
```

### Swift (iOS/macOS)

```bash
# Build iOS app
halvor build ios

# Build macOS app
halvor build mac
```

### Android

```bash
# Build Android library and app
halvor build android
```

### Web (WASM + SvelteKit)

```bash
# Build web application
halvor build web
```

## Development Workflows

### CLI Development

```bash
# Development mode with auto-rebuild
halvor dev cli
# or
make dev
```

### Swift Development

```bash
# macOS development with hot reload
halvor dev mac

# iOS simulator
halvor dev ios
```

### Web Development

```bash
# Docker-based development (recommended)
halvor dev web

# Bare-metal development (Rust server + Svelte dev)
halvor dev web --bare-metal

# Production mode
halvor dev web --prod
```

### Android Development

Build JNI once, then use Android Studio for app development:

```bash
halvor build android
```

## Generated Files

- **Swift**: `halvor-swift/Sources/HalvorSwiftFFI/halvor_ffi/generated_swift_bindings.swift`
- **Kotlin**: `halvor-android/src/main/kotlin/dev/scottkey/halvor/GeneratedBindings.kt`
- **TypeScript**: `halvor-web/src/lib/halvor-ffi/generated-bindings.ts`

All generated files are automatically included in their respective build systems.

## Adding FFI-Exported Functions

1. Implement function in service module
2. Annotate with `#[multi_platform_export]`
3. Rebuild platform bindings:
   - Swift: `cd halvor-swift && ./build.sh`
   - Android: `halvor build android`
   - Web: `halvor build web`

## Platform Support

- **macOS**: Full support for all platforms (CLI, iOS, macOS, Android, Web)
- **Linux**: CLI and Web development (iOS/macOS builds not available)
- **Windows**: CLI only (via WSL recommended)

For detailed build instructions, see the [Development Guide](development.md).

