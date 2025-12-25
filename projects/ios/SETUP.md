# Setup Guide for HalvorSwift

This guide will help you set up and build the HalvorSwift package.

## Prerequisites

1. **Rust toolchain** - Install from [rustup.rs](https://rustup.rs/)
2. **Xcode Command Line Tools** - Install via `xcode-select --install`
3. **Swift 5.9+** - Usually comes with Xcode

## Initial Setup

### 1. Install Rust Targets

The build script will automatically install iOS targets, but you can also install them manually:

```bash
make install-targets
```

Or manually:
```bash
rustup target add aarch64-apple-darwin
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-ios
rustup target add x86_64-apple-ios
rustup target add aarch64-apple-ios-sim
rustup target add x86_64-apple-ios-sim
```

### 2. Install UniFFI Bindgen

UniFFI bindgen is required to generate Swift bindings from the Rust code:

```bash
make install-uniffi
```

Or manually:
```bash
# No installation needed - uniffi-bindgen is built from the halvor-ffi crate
# Or install manually: cargo install --git https://github.com/mozilla/uniffi-rs.git uniffi_bindgen
```

### 3. Build the Package

Build the Rust FFI library and create the XCFramework:

```bash
make build
```

Or run the build script directly:
```bash
./build.sh
```

This will:
- Generate UniFFI Swift bindings
- Build the Rust library for all iOS and macOS targets
- Create an XCFramework containing all platform variants

## Project Structure

```
halvor-swift/
├── Package.swift              # Swift Package Manager manifest
├── Sources/
│   ├── HalvorSwift/          # Swift wrapper code
│   │   └── HalvorClient.swift
│   └── HalvorSwiftFFI/       # Generated FFI bindings and XCFramework
│       ├── HalvorSwiftFFI.xcframework
│       ├── HalvorSwiftFFI.swift (generated)
│       └── HalvorSwiftFFI.h (generated)
├── halvor-ffi/               # Rust FFI library
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   └── halvor_ffi.udl    # UniFFI interface definition
│   └── uniffi-bindgen.toml
├── Examples/                 # Example Swift code
├── build.sh                  # Build script
└── Makefile                  # Convenience commands
```

## Using in Your Project

### Swift Package Manager

Add to your `Package.swift`:

```swift
dependencies: [
    .package(path: "../halvor-swift")  // Local path
    // Or from a git repository:
    // .package(url: "https://github.com/yourusername/halvor-swift.git", from: "0.0.6")
]
```

### Xcode

1. File → Add Packages...
2. Enter the repository URL or select a local path
3. Add `HalvorSwift` to your target's dependencies

## Troubleshooting

### Build Errors

**Error: "cargo is not installed"**
- Install Rust from [rustup.rs](https://rustup.rs/)

**Error: "uniffi-bindgen not found"**
- Run `make install-uniffi` or `# No installation needed - uniffi-bindgen is built from the halvor-ffi crate
# Or install manually: cargo install --git https://github.com/mozilla/uniffi-rs.git uniffi_bindgen`

**Error: "target not found"**
- Run `make install-targets` to install all required Rust targets

**Error: "Library not found"**
- Make sure you've run `./build.sh` to build the Rust library
- Check that the parent `halvor` crate is accessible (the FFI crate depends on it)

### XCFramework Issues

If the XCFramework creation fails:
- Make sure Xcode Command Line Tools are installed
- Check that `xcodebuild` is available: `xcodebuild -version`
- The build script includes a fallback manual XCFramework creation

### Linking Issues

If you get linking errors in your Swift project:
- Make sure the XCFramework is properly included in your target
- Check that you're using the correct import: `import HalvorSwift`
- Verify the XCFramework contains the architecture you need (arm64, x86_64, etc.)

## Development Workflow

1. **Make changes to Rust FFI code** (`halvor-ffi/src/lib.rs` or `halvor_ffi.udl`)
2. **Rebuild**: Run `./build.sh` or `make build`
3. **Test**: Use the package in your app or run `swift test`

## Notes

- The build process generates Swift bindings automatically from the UDL file
- The XCFramework includes both iOS and macOS variants
- Universal binaries are created for macOS (arm64 + x86_64)
- The package requires the parent `halvor` crate to be available (via path dependency)
