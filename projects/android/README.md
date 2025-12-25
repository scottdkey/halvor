# Halvor Android

Android library for Halvor using JNI bindings from Rust.

## Setup

1. Install Android NDK
2. Install Rust Android targets:
```bash
rustup target add aarch64-linux-android
rustup target add armv7-linux-androideabi
rustup target add i686-linux-android
rustup target add x86_64-linux-android
```

3. Build JNI library:
```bash
make android-jni-build
```

4. Build Android library:
```bash
make android-build
```

## Usage

```kotlin
import dev.scottkey.halvor.HalvorClient

val client = HalvorClient()
val hosts = client.discoverAgents()
```

## Architecture

- **Rust FFI**: JNI bindings via `jni` crate
- **Kotlin Wrapper**: High-level API using JNA
- **Auto-generated**: Bindings generated from `#[kotlin_export]` macros

