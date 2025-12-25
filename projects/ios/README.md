# HalvorSwift

Swift Package Manager package for iOS and macOS apps that use Halvor's Rust core logic via a Swift bridge.

## Overview

This package provides Swift bindings to the Halvor homelab automation layer, allowing iOS and macOS apps to:
- Discover Halvor agents on the network
- Query agent status and host information
- Execute commands on remote agents
- Manage homelab infrastructure from Swift applications

## Requirements

- Swift 5.9+
- iOS 13.0+ or macOS 10.15+
- Rust toolchain (for building)
- Xcode Command Line Tools

## Prerequisites

**Important**: You must build the package first before using it. The `HalvorSwiftFFI` module is generated during the build process.

```bash
cd halvor-swift
make setup    # Install Rust targets and UniFFI
make build    # Generate Swift bindings and build XCFramework
```

Or from the project root:
```bash
make swift-setup
make swift-build
```

## Installation

### Using Swift Package Manager

Add this package to your `Package.swift`:

```swift
dependencies: [
    .package(url: "https://github.com/yourusername/halvor-swift.git", from: "0.0.6")
]
```

Or add it via Xcode:
1. File → Add Packages...
2. Enter the repository URL
3. Select version or branch

## Building

Before using the package, you need to build the Rust FFI library:

```bash
cd halvor-swift
./build.sh
```

This will:
1. Generate UniFFI Swift bindings from the Rust code
2. Build the Rust library for iOS and macOS targets
3. Create an XCFramework containing all platform variants

## Usage

### Basic Example

```swift
import HalvorSwift

// Create a client
let client = HalvorClient()

// Discover agents
do {
    let hosts = try client.discoverAgents()
    print("Discovered \(hosts.count) agents")
    
    for host in hosts {
        print("Host: \(host.displayName)")
        print("IP: \(host.primaryIP ?? "unknown")")
        print("Reachable: \(host.reachable)")
        
        // Get host info
        if let ip = host.primaryIP {
            let info = try client.getHostInfo(host: ip)
            print("Docker: \(info.dockerVersion ?? "unknown")")
            print("Portainer: \(info.portainerInstalled ? "installed" : "not installed")")
        }
    }
} catch {
    print("Error: \(error)")
}
```

### Async/Await Example

```swift
import HalvorSwift

Task {
    let client = HalvorClient()
    
    do {
        // Discover agents asynchronously
        let hosts = try await Task {
            try client.discoverAgents()
        }.value
        
        // Process results
        for host in hosts {
            if host.reachable {
                print("\(host.displayName) is reachable")
            }
        }
    } catch {
        print("Discovery failed: \(error)")
    }
}
```

### Error Handling

```swift
import HalvorSwift

let client = HalvorClient()

do {
    let hosts = try client.discoverAgents()
} catch HalvorError.connectionFailed(let message) {
    print("Connection failed: \(message)")
} catch HalvorError.agentError(let message) {
    print("Agent error: \(message)")
} catch HalvorError.discoveryError(let message) {
    print("Discovery error: \(message)")
} catch {
    print("Unknown error: \(error)")
}
```

## API Reference

### HalvorClient

Main client class for interacting with Halvor agents.

#### Methods

- `init(agentPort: UInt16?)` - Initialize client with optional agent port (default: 23500)
- `discoverAgents() throws -> [DiscoveredHost]` - Discover all available agents
- `discoverViaTailscale() throws -> [DiscoveredHost]` - Discover agents via Tailscale
- `discoverViaLocalNetwork() throws -> [DiscoveredHost]` - Discover agents on local network
- `pingAgent(host: String, port: UInt16) throws -> Bool` - Check if agent is reachable
- `getHostInfo(host: String, port: UInt16) throws -> HostInfo` - Get host information
- `executeCommand(host: String, port: UInt16, command: String, args: [String]) throws -> String` - Execute command on remote agent

### DiscoveredHost

Represents a discovered host on the network.

#### Properties

- `hostname: String` - Hostname of the discovered host
- `localIP: String?` - Local network IP address
- `tailscaleIP: String?` - Tailscale IP address
- `tailscaleHostname: String?` - Tailscale hostname
- `agentPort: UInt16` - Port the agent is listening on
- `reachable: Bool` - Whether the agent is currently reachable
- `primaryIP: String?` - Primary IP (prefers Tailscale if available)
- `displayName: String` - Display name for the host

### HostInfo

Information about a host retrieved from an agent.

#### Properties

- `dockerVersion: String?` - Docker version installed on the host
- `tailscaleInstalled: Bool` - Whether Tailscale is installed
- `portainerInstalled: Bool` - Whether Portainer is installed

### HalvorError

Error types for Halvor operations.

#### Cases

- `connectionFailed(String)` - Failed to connect to agent
- `agentError(String)` - Agent returned an error
- `discoveryError(String)` - Discovery operation failed
- `unknown(String)` - Unknown error

## Development

### Project Structure

```
halvor-swift/
├── Package.swift              # Swift Package Manager manifest
├── Sources/
│   ├── HalvorSwift/          # Swift wrapper code
│   │   └── HalvorClient.swift
│   └── HalvorSwiftFFI/       # Generated FFI bindings and XCFramework
│       └── HalvorSwiftFFI.xcframework
├── halvor-ffi/               # Rust FFI library
│   ├── Cargo.toml
│   ├── src/
│   │   └── lib.rs
│   └── uniffi-bindgen.toml
└── build.sh                  # Build script
```

### Rebuilding

After making changes to the Rust FFI code:

```bash
./build.sh
```

The build script will:
1. Generate new Swift bindings
2. Rebuild the Rust library for all targets
3. Recreate the XCFramework

### Testing

Create a test target in your app or add tests to the package:

```swift
import XCTest
import HalvorSwift

final class HalvorSwiftTests: XCTestCase {
    func testDiscovery() throws {
        let client = HalvorClient()
        let hosts = try client.discoverAgents()
        // Add assertions
    }
}
```

## License

MIT License - see LICENSE file for details
