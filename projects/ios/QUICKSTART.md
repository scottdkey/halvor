# Quick Start Guide

Get started with HalvorSwift in 5 minutes!

## 1. Prerequisites

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Xcode Command Line Tools (if not already installed)
xcode-select --install
```

## 2. Setup

```bash
cd halvor-swift

# Install Rust targets and UniFFI
make setup

# Or manually:
make install-targets
make install-uniffi
```

## 3. Build

```bash
# Build the Swift bridge
make build

# Or:
./build.sh
```

This creates the XCFramework that Swift Package Manager will use.

## 4. Use in Your Project

### In Xcode:

1. File → Add Packages...
2. Click "+" and select "Add Local..."
3. Navigate to the `halvor-swift` directory
4. Click "Add Package"
5. Select `HalvorSwift` when adding to your target

### In Swift Package Manager:

Add to your `Package.swift`:

```swift
dependencies: [
    .package(path: "../halvor-swift")
]
```

## 5. Example Usage

```swift
import HalvorSwift

let client = HalvorClient()

// Discover agents
do {
    let hosts = try client.discoverAgents()
    for host in hosts {
        print("Found: \(host.displayName) at \(host.primaryIP ?? "unknown")")
    }
} catch {
    print("Error: \(error)")
}
```

That's it! You're ready to use Halvor from Swift.

## Next Steps

- See `README.md` for full API documentation
- Check `Examples/` for more examples
- Read `SETUP.md` for detailed setup instructions
