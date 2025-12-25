# Swift Export Macro - Usage Guide

## Overview

The `#[swift_export]` macro automatically generates Swift bindings from Rust functions. When you rebuild the Rust code, Swift bindings are automatically generated and available in your Swift code.

## Quick Start

1. **Add the macro to your Rust function:**

```rust
use halvor_ffi_macro::swift_export;

#[swift_export]
pub fn my_new_function(param: String) -> Result<String, String> {
    Ok(format!("Hello, {}!", param))
}
```

2. **Rebuild the Rust code:**

```bash
cd halvor-swift/halvor-ffi
cargo build --release --target aarch64-apple-darwin
```

3. **The Swift binding is automatically generated!**

The function is now available in Swift:

```swift
let result = try myNewFunction(param: "World")
print(result) // "Hello, World!"
```

## How It Works

1. **Macro Annotation**: You mark functions with `#[swift_export]`
2. **Build Script**: The `build.rs` script scans your Rust code during compilation
3. **Code Generation**: Swift bindings are generated in `Sources/HalvorSwiftFFI/halvor_ffi/generated_swift_bindings.swift`
4. **Automatic Integration**: The Swift package automatically includes the generated file

## Example: Adding a New Function

### Step 1: Write the Rust Function

```rust
// In halvor-ffi/src/lib.rs

#[swift_export]
pub fn get_server_uptime(host: String) -> Result<u64, String> {
    // Your implementation
    Ok(12345)
}
```

### Step 2: Rebuild

```bash
make build-mac  # or make build-ios
```

### Step 3: Use in Swift

```swift
let uptime = try client.getServerUptime(host: "example.com")
```

## Type Mapping

| Rust Type           | Swift Type                   |
| ------------------- | ---------------------------- |
| `String`            | `String`                     |
| `u16`, `u32`, `u64` | `UInt16`, `UInt32`, `UInt64` |
| `i32`, `i64`        | `Int32`, `Int64`             |
| `bool`              | `Bool`                       |
| `Result<T, E>`      | `throws -> T`                |
| `Vec<T>`            | `[T]`                        |
| `Option<T>`         | `T?`                         |

## Struct Export

Structs with `#[derive(Serialize, Deserialize)]` are automatically exported:

```rust
#[derive(Serialize, Deserialize)]
pub struct ServerStatus {
    pub online: bool,
    pub uptime: u64,
    pub version: String,
}

#[swift_export]
pub fn get_status() -> Result<ServerStatus, String> {
    Ok(ServerStatus {
        online: true,
        uptime: 12345,
        version: "1.0.0".to_string(),
    })
}
```

Swift automatically gets:
```swift
struct ServerStatus: Codable {
    let online: Bool
    let uptime: UInt64
    let version: String
}
```

## Integration with Build Process

The macro system is integrated into the existing build process:

- `make build-mac` - Builds Rust and generates Swift bindings for macOS
- `make build-ios` - Builds Rust and generates Swift bindings for iOS
- `make dev-mac` - Development mode with auto-rebuild on changes
- `make dev-ios` - Development mode for iOS with auto-rebuild

Every time Rust code is rebuilt, Swift bindings are automatically regenerated!
