use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, parse_macro_input};

/// Macro to mark a Rust function for multi-platform export (Swift, Kotlin, WASM)
///
/// This macro:
/// 1. Keeps the original function unchanged
/// 2. Generates metadata for build scripts to create bindings for all platforms
/// 3. Supports platform-specific attributes
///
/// Example:
/// ```rust
/// #[swift_export]
/// pub fn discover_agents(client: &HalvorClient) -> Result<Vec<DiscoveredHost>, String> {
///     client.discover_agents()
/// }
/// ```
///
/// Platform-specific attributes:
/// - `#[swift_export(rename = "newName")]` - Rename for Swift
/// - `#[swift_export(kotlin_name = "newName")]` - Rename for Kotlin
/// - `#[swift_export(wasm_name = "newName")]` - Rename for WASM
#[proc_macro_attribute]
pub fn swift_export(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);

    // For now, just pass through - the build script will handle generation
    // In the future, we could generate FFI wrappers here too
    TokenStream::from(quote! {
        #input_fn
    })
}

/// Macro to mark a Rust function for Kotlin/JNI export
#[proc_macro_attribute]
pub fn kotlin_export(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);
    TokenStream::from(quote! {
        #input_fn
    })
}

/// Macro to mark a Rust function for WASM export
#[proc_macro_attribute]
pub fn wasm_export(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);
    TokenStream::from(quote! {
        #input_fn
    })
}

/// Macro to mark a Rust function for all platforms (Swift, Kotlin, WASM)
#[proc_macro_attribute]
pub fn multi_platform_export(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);
    TokenStream::from(quote! {
        #input_fn
    })
}
