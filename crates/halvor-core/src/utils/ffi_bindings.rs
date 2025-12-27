// FFI binding generation utilities
// This module contains functions to generate platform-specific bindings from Rust FFI code

use quote::ToTokens;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Generate FFI bindings (for use in build.rs or CLI)
pub fn generate_ffi_bindings() {
    if let Err(e) = generate_ffi_bindings_cli() {
        eprintln!("Warning: Failed to generate FFI bindings: {}", e);
    }
}

/// Generate FFI bindings (CLI version with proper error handling)
pub fn generate_ffi_bindings_cli() -> Result<()> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| {
        env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string())
    });
    let ffi_dir = Path::new("src/ffi");

    if !ffi_dir.exists() {
        return Ok(());
    }

    // Use println! for CLI, cargo:warning for build.rs
    let is_build_script = env::var("OUT_DIR").is_ok();
    if is_build_script {
        println!("cargo:warning=FFI bindings generation - scanning src/ffi/");
    } else {
        println!("FFI bindings generation - scanning src/ffi/");
    }

    // Collect Rust files
    let mut rust_files = Vec::new();
    collect_rust_files(ffi_dir, &mut rust_files);

    // Parse and extract
    let mut functions = Vec::new();
    let mut structs = Vec::new();

    let is_build_script = env::var("OUT_DIR").is_ok();
    let log = |msg: &str| {
        if is_build_script {
            println!("cargo:warning={}", msg);
        } else {
            println!("{}", msg);
        }
    };

    for file_path in &rust_files {
        log(&format!("Reading file: {:?}", file_path));
        if let Ok(content) = fs::read_to_string(file_path) {
            if let Ok(ast) = syn::parse_file(&content) {
                extract_ffi_items(&ast, &mut functions, &mut structs, is_build_script);
            } else {
                log(&format!("Failed to parse file: {:?}", file_path));
            }
        } else {
            log(&format!("Failed to read file: {:?}", file_path));
        }
    }

    log(&format!("Found {} FFI functions", functions.len()));

    // Generate bindings
    if !functions.is_empty() {
        generate_swift_bindings(&manifest_dir, &functions, &structs)?;
        generate_kotlin_bindings(&manifest_dir, &functions, &structs)?;
        generate_wasm_bindings(&manifest_dir, &functions, &structs)?;
    }

    Ok(())
}

fn collect_rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_dir() {
                    collect_rust_files(&path, files);
                } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                    files.push(path);
                }
            }
        }
    }
}

fn extract_function_from_impl_item(
    fn_item: &syn::ImplItemFn,
    functions: &mut Vec<(String, Vec<String>)>,
) {
    // syn and quote are build-dependencies, available when included in build.rs

    let fn_name = fn_item.sig.ident.to_string();
    let mut platforms = Vec::new();

    for attr in &fn_item.attrs {
        let path_str = attr.path().to_token_stream().to_string();
        let is_swift_export =
            path_str.contains("swift_export") || path_str.contains("multi_platform_export");
        let is_kotlin_export =
            path_str.contains("kotlin_export") || path_str.contains("multi_platform_export");
        let is_wasm_export =
            path_str.contains("wasm_export") || path_str.contains("multi_platform_export");

        if is_swift_export && !platforms.contains(&"swift".to_string()) {
            platforms.push("swift".to_string());
        }
        if is_kotlin_export && !platforms.contains(&"kotlin".to_string()) {
            platforms.push("kotlin".to_string());
        }
        if is_wasm_export && !platforms.contains(&"wasm".to_string()) {
            platforms.push("wasm".to_string());
        }
    }

    if !platforms.is_empty() {
        println!(
            "cargo:warning=Found FFI function: {} for platforms: {:?}",
            fn_name, platforms
        );
        functions.push((fn_name, platforms));
    }
}

fn extract_function_from_item_fn(
    fn_item: &syn::ItemFn,
    functions: &mut Vec<(String, Vec<String>)>,
) {
    // syn and quote are build-dependencies, available when included in build.rs

    let fn_name = fn_item.sig.ident.to_string();
    let mut platforms = Vec::new();

    for attr in &fn_item.attrs {
        let path_str = attr.path().to_token_stream().to_string();
        let is_swift_export =
            path_str.contains("swift_export") || path_str.contains("multi_platform_export");
        let is_kotlin_export =
            path_str.contains("kotlin_export") || path_str.contains("multi_platform_export");
        let is_wasm_export =
            path_str.contains("wasm_export") || path_str.contains("multi_platform_export");

        if is_swift_export && !platforms.contains(&"swift".to_string()) {
            platforms.push("swift".to_string());
        }
        if is_kotlin_export && !platforms.contains(&"kotlin".to_string()) {
            platforms.push("kotlin".to_string());
        }
        if is_wasm_export && !platforms.contains(&"wasm".to_string()) {
            platforms.push("wasm".to_string());
        }
    }

    if !platforms.is_empty() {
        println!(
            "cargo:warning=Found FFI function: {} for platforms: {:?}",
            fn_name, platforms
        );
        functions.push((fn_name, platforms));
    }
}

fn extract_ffi_items(
    ast: &syn::File,
    functions: &mut Vec<(String, Vec<String>)>, // (name, platforms)
    structs: &mut Vec<(String, Vec<(String, String)>)>, // (name, fields)
    is_build_script: bool,
) {
    let log = |msg: &str| {
        if is_build_script {
            println!("cargo:warning={}", msg);
        }
    };

    log(&format!(
        "Scanning {} items for FFI functions",
        ast.items.len()
    ));

    for item in &ast.items {
        // Handle functions in impl blocks
        if let syn::Item::Impl(impl_item) = item {
            for impl_item in &impl_item.items {
                if let syn::ImplItem::Fn(fn_item) = impl_item {
                    extract_function_from_impl_item(fn_item, functions);
                }
            }
        }

        // Handle standalone functions
        if let syn::Item::Fn(fn_item) = item {
            extract_function_from_item_fn(fn_item, functions);
        }

        // Extract structs with Serialize
        if let syn::Item::Struct(struct_item) = item {
            let has_serialize = struct_item.attrs.iter().any(|attr| {
                if attr.path().is_ident("derive") {
                    let tokens = quote::ToTokens::to_token_stream(attr).to_string();
                    return tokens.contains("Serialize");
                }
                false
            });

            if has_serialize {
                let struct_name = struct_item.ident.to_string();
                let mut fields = Vec::new();

                if let syn::Fields::Named(named_fields) = &struct_item.fields {
                    for field in &named_fields.named {
                        if let Some(ident) = &field.ident {
                            let field_name = ident.to_string();
                            let field_type = format!("{:?}", field.ty);
                            fields.push((field_name, field_type));
                        }
                    }
                }

                structs.push((struct_name, fields));
            }
        }
    }
}

fn generate_swift_bindings(
    manifest_dir: &str,
    functions: &[(String, Vec<String>)],
    structs: &[(String, Vec<(String, String)>)],
) -> Result<()> {
    let swift_funcs: Vec<_> = functions
        .iter()
        .filter(|(_, platforms)| platforms.contains(&"swift".to_string()))
        .collect();

    let mut code = String::from("// Auto-generated Swift bindings from Rust\n");
    code.push_str("// DO NOT EDIT - This file is generated automatically\n\n");
    code.push_str("import Foundation\n\n");

    for (struct_name, fields) in structs {
        code.push_str(&format!("public struct {}: Codable {{\n", struct_name));
        for (field_name, _) in fields {
            let swift_field = to_camel_case(field_name);
            code.push_str(&format!("    public let {}: String?\n", swift_field));
        }
        code.push_str("}\n\n");
    }

    code.push_str("// Functions:\n");
    for (func_name, _) in swift_funcs {
        code.push_str(&format!("// - {}\n", func_name));
    }

    let output_dir = PathBuf::from(manifest_dir)
        .join("projects")
        .join("ios")
        .join("Sources")
        .join("HalvorSwiftFFI")
        .join("halvor_ffi");

    fs::create_dir_all(&output_dir)
        .with_context(|| format!("Failed to create Swift output directory: {:?}", output_dir))?;
    fs::write(output_dir.join("generated_swift_bindings.swift"), code)
        .with_context(|| "Failed to write Swift bindings")?;
    Ok(())
}

fn generate_kotlin_bindings(
    manifest_dir: &str,
    functions: &[(String, Vec<String>)],
    structs: &[(String, Vec<(String, String)>)],
) -> Result<()> {
    let kotlin_funcs: Vec<_> = functions
        .iter()
        .filter(|(_, platforms)| platforms.contains(&"kotlin".to_string()))
        .collect();

    let mut code = String::from("// Auto-generated Kotlin bindings\n");
    code.push_str("package dev.scottkey.halvor\n\n");

    for (struct_name, fields) in structs {
        code.push_str(&format!("data class {}(\n", struct_name));
        for (i, (field_name, _)) in fields.iter().enumerate() {
            let kotlin_field = to_camel_case(field_name);
            let comma = if i < fields.len() - 1 { "," } else { "" };
            code.push_str(&format!("    val {}: String?{}\n", kotlin_field, comma));
        }
        code.push_str(")\n\n");
    }

    code.push_str("// Functions:\n");
    for (func_name, _) in kotlin_funcs {
        code.push_str(&format!("// - {}\n", func_name));
    }

    let output_dir = PathBuf::from(manifest_dir)
        .join("projects")
        .join("android")
        .join("src")
        .join("main")
        .join("kotlin")
        .join("dev")
        .join("scottkey")
        .join("halvor");

    fs::create_dir_all(&output_dir)
        .with_context(|| format!("Failed to create Kotlin output directory: {:?}", output_dir))?;
    fs::write(output_dir.join("GeneratedBindings.kt"), code)
        .with_context(|| "Failed to write Kotlin bindings")?;
    Ok(())
}

fn generate_wasm_bindings(
    manifest_dir: &str,
    functions: &[(String, Vec<String>)],
    structs: &[(String, Vec<(String, String)>)],
) -> Result<()> {
    let wasm_funcs: Vec<_> = functions
        .iter()
        .filter(|(_, platforms)| platforms.contains(&"wasm".to_string()))
        .collect();

    let mut code = String::from("// Auto-generated TypeScript bindings\n\n");

    for (struct_name, fields) in structs {
        code.push_str(&format!("export interface {} {{\n", struct_name));
        for (field_name, _) in fields {
            let ts_field = to_camel_case(field_name);
            code.push_str(&format!("    {}?: string;\n", ts_field));
        }
        code.push_str("}\n\n");
    }

    code.push_str("export interface HalvorWasmModule {\n");
    for (func_name, _) in wasm_funcs {
        let ts_name = to_camel_case(func_name);
        code.push_str(&format!("    {}(): Promise<any>;\n", ts_name));
    }
    code.push_str("}\n");

    let output_dir = PathBuf::from(manifest_dir)
        .join("projects")
        .join("web")
        .join("src")
        .join("lib")
        .join("halvor-ffi");

    fs::create_dir_all(&output_dir)
        .with_context(|| format!("Failed to create WASM output directory: {:?}", output_dir))?;
    fs::write(output_dir.join("generated-bindings.ts"), code)
        .with_context(|| "Failed to write WASM TypeScript bindings")?;
    Ok(())
}

fn to_camel_case(snake: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;

    for c in snake.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_uppercase().next().unwrap());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }

    result
}
