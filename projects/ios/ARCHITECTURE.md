# Architecture Overview

This document explains the architecture of the HalvorSwift bridge.

## High-Level Architecture

```
┌─────────────────┐
│  Swift App      │
│  (iOS/macOS)    │
└────────┬────────┘
         │
         │ Swift API
         │
┌────────▼────────┐
│  HalvorSwift    │
│  (Swift Wrapper)│
└────────┬────────┘
         │
         │ FFI Calls
         │
┌────────▼────────┐
│ HalvorSwiftFFI  │
│ (UniFFI Bindings)│
└────────┬────────┘
         │
         │ C ABI
         │
┌────────▼────────┐
│  halvor-ffi     │
│  (Rust Library) │
└────────┬────────┘
         │
         │ Rust API
         │
┌────────▼────────┐
│     halvor      │
│  (Core Library) │
└─────────────────┘
```

## Components

### 1. HalvorSwift (Swift Wrapper)

**Location**: `Sources/HalvorSwift/HalvorClient.swift`

Provides a Swift-idiomatic API that wraps the generated UniFFI bindings. This layer:
- Converts FFI types to Swift-native types
- Provides convenience properties (e.g., `primaryIP`, `displayName`)
- Handles error conversion
- Offers a cleaner API surface

### 2. HalvorSwiftFFI (UniFFI Generated)

**Location**: `Sources/HalvorSwiftFFI/` (generated during build)

Auto-generated Swift bindings from the UniFFI UDL file. This includes:
- `HalvorSwiftFFI.swift` - Swift bindings
- `HalvorSwiftFFI.h` - C header (if needed)
- `HalvorSwiftFFI.xcframework` - Compiled Rust library

### 3. halvor-ffi (Rust FFI Library)

**Location**: `halvor-ffi/`

Rust crate that:
- Defines the FFI interface in `halvor_ffi.udl` (UniFFI Definition Language)
- Implements the interface in `src/lib.rs`
- Bridges to the core `halvor` crate
- Compiles to static libraries for each platform

### 4. halvor (Core Library)

**Location**: `../../` (parent directory)

The main Halvor Rust crate that provides:
- Agent discovery
- Agent client API
- Host information management
- All core homelab automation logic

## Data Flow

### Discovery Flow

```
Swift App
  ↓
HalvorClient.discoverAgents()
  ↓
HalvorSwiftFFI.HalvorClient.discoverAgents()
  ↓ (FFI call)
halvor_ffi::HalvorClient::discover_agents()
  ↓
halvor::agent::discovery::HostDiscovery::discover_all()
  ↓
Returns Vec<DiscoveredHost>
  ↓ (converted)
Returns [HalvorSwiftFFI.DiscoveredHost]
  ↓ (wrapped)
Returns [HalvorSwift.DiscoveredHost]
```

### Error Handling

Errors flow through the same layers:
1. Rust `Result<T, E>` → UniFFI error enum
2. UniFFI error enum → Swift `HalvorSwiftFFI.HalvorError`
3. Swift FFI error → Swift `HalvorSwift.HalvorError`

## Build Process

1. **UniFFI Binding Generation**
   - Reads `halvor_ffi.udl`
   - Generates Swift bindings
   - Creates scaffolding code

2. **Rust Compilation**
   - Compiles `halvor-ffi` for each target:
     - `aarch64-apple-darwin` (macOS ARM)
     - `x86_64-apple-darwin` (macOS Intel)
     - `aarch64-apple-ios` (iOS device)
     - `aarch64-apple-ios-sim` (iOS simulator ARM)
     - `x86_64-apple-ios` (iOS simulator Intel)

3. **Framework Creation**
   - Creates `.framework` for each platform
   - Combines macOS architectures with `lipo`
   - Assembles into `.xcframework`

4. **Package Integration**
   - XCFramework is included as binary target
   - Swift Package Manager handles linking

## Type Mapping

| Rust Type        | UniFFI Type    | Swift FFI Type       | Swift Wrapper Type   |
| ---------------- | -------------- | -------------------- | -------------------- |
| `String`         | `string`       | `String`             | `String`             |
| `Option<String>` | `string?`      | `String?`            | `String?`            |
| `Vec<T>`         | `sequence<T>`  | `[T]`                | `[T]`                |
| `bool`           | `boolean`      | `Bool`               | `Bool`               |
| `u16`            | `u16`          | `UInt16`             | `UInt16`             |
| `Result<T, E>`   | `throws Error` | `throws HalvorError` | `throws HalvorError` |

## Threading Model

- UniFFI handles thread safety automatically
- Rust code can be called from any Swift thread
- Internal Rust code uses `tokio` for async operations
- Swift async/await can be used with synchronous FFI calls

## Memory Management

- UniFFI handles memory management automatically
- Rust objects are reference-counted in Swift
- No manual memory management required
- Objects are automatically deallocated when no longer referenced

## Platform Support

- **iOS 13.0+**: Device and Simulator (arm64, x86_64)
- **macOS 10.15+**: Universal binary (arm64 + x86_64)

## Future Enhancements

Potential improvements:
- Async Rust operations exposed as Swift async/await
- More comprehensive error types
- Streaming support for long-running operations
- Additional agent operations (backup, restore, etc.)
