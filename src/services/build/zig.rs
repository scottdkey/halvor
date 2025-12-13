// Zig cross-compilation setup
use anyhow::{Context, Result};
use std::process::Command;
use which;

/// Set up Zig for cross-compilation to Linux and Windows targets
pub fn setup_zig_cross_compilation(cmd: &mut Command, target: &str) -> Result<()> {
    // Check if Zig is installed
    let zig_check = Command::new("zig").args(["version"]).output();

    if zig_check.is_err() || !zig_check.unwrap().status.success() {
        println!("  ⚠️  Zig not found. Installing Zig for cross-compilation...");

        // Try to install Zig via homebrew on macOS
        let brew_install = Command::new("brew").args(["install", "zig"]).status();

        if brew_install.is_err() || !brew_install.unwrap().success() {
            eprintln!("  ❌ Failed to install Zig. Please install manually:");
            eprintln!("     brew install zig");
            eprintln!("  Or download from: https://ziglang.org/download/");
            anyhow::bail!("Zig is required for cross-compilation from macOS");
        }

        println!("  ✓ Zig installed successfully");
    }

    // Convert Rust target triple to Zig target format
    let zig_target = rust_target_to_zig_target(target);

    // Create wrapper scripts for CC, CXX, and AR
    let temp_dir = std::env::temp_dir();
    let target_safe = target.replace("-", "_");

    let cc_wrapper_name = format!("zig-cc-{}", target_safe);
    let cxx_wrapper_name = format!("zig-cxx-{}", target_safe);
    let ar_wrapper_name = format!("zig-ar-{}", target_safe);

    let cc_wrapper_path = temp_dir.join(&cc_wrapper_name);
    let cxx_wrapper_path = temp_dir.join(&cxx_wrapper_name);
    let ar_wrapper_path = temp_dir.join(&ar_wrapper_name);

    // Create CC wrapper script
    // Use absolute path to zig to ensure it's found
    let zig_path = which::which("zig")
        .context("Zig not found in PATH")?
        .to_string_lossy()
        .to_string();

    // For gnu targets, try to use GNU ld (bfd) which supports --allow-multiple-definition
    // This is needed to resolve FFI symbol conflicts (halvor_string_free duplicate)
    // For musl targets, use LLD (Zig's default)
    let linker_flag = if target.contains("linux-gnu") {
        // Try to find binutils via brew --prefix
        let binutils_prefix = Command::new("brew")
            .args(["--prefix", "binutils"])
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    String::from_utf8(o.stdout)
                        .ok()
                        .map(|s| s.trim().to_string())
                } else {
                    None
                }
            });

        if let Some(prefix) = binutils_prefix {
            let ld_path = std::path::Path::new(&prefix).join("bin").join("ld");
            if ld_path.exists() {
                format!("-fuse-ld={}", ld_path.display())
            } else {
                String::new()
            }
        } else if std::path::Path::new("/opt/homebrew/opt/binutils/bin/ld").exists() {
            "-fuse-ld=/opt/homebrew/opt/binutils/bin/ld".to_string()
        } else if std::path::Path::new("/usr/local/opt/binutils/bin/ld").exists() {
            "-fuse-ld=/usr/local/opt/binutils/bin/ld".to_string()
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let cc_wrapper_content = if !linker_flag.is_empty() {
        format!(
            "#!/bin/sh\nexec {} cc -target {} {} \"$@\"\n",
            zig_path, zig_target, linker_flag
        )
    } else {
        format!(
            "#!/bin/sh\nexec {} cc -target {} \"$@\"\n",
            zig_path, zig_target
        )
    };
    std::fs::write(&cc_wrapper_path, cc_wrapper_content)
        .context("Failed to create Zig CC wrapper script")?;

    // Create CXX wrapper script
    let cxx_wrapper_content = if !linker_flag.is_empty() {
        format!(
            "#!/bin/sh\nexec {} c++ -target {} {} \"$@\"\n",
            zig_path, zig_target, linker_flag
        )
    } else {
        format!(
            "#!/bin/sh\nexec {} c++ -target {} \"$@\"\n",
            zig_path, zig_target
        )
    };
    std::fs::write(&cxx_wrapper_path, cxx_wrapper_content)
        .context("Failed to create Zig CXX wrapper script")?;

    // Create AR wrapper script
    let ar_wrapper_content = format!("#!/bin/sh\nexec {} ar \"$@\"\n", zig_path);
    std::fs::write(&ar_wrapper_path, ar_wrapper_content)
        .context("Failed to create Zig AR wrapper script")?;

    // Make all wrapper scripts executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(&cc_wrapper_path, perms.clone())
            .context("Failed to make CC wrapper script executable")?;
        std::fs::set_permissions(&cxx_wrapper_path, perms.clone())
            .context("Failed to make CXX wrapper script executable")?;
        std::fs::set_permissions(&ar_wrapper_path, perms)
            .context("Failed to make AR wrapper script executable")?;
    }

    // Get wrapper paths as strings
    let cc_wrapper_str = cc_wrapper_path
        .to_str()
        .context("CC wrapper path contains invalid UTF-8")?;
    let cxx_wrapper_str = cxx_wrapper_path
        .to_str()
        .context("CXX wrapper path contains invalid UTF-8")?;
    let ar_wrapper_str = ar_wrapper_path
        .to_str()
        .context("AR wrapper path contains invalid UTF-8")?;

    // Set environment variables for cc-rs
    // cc-rs looks for CC_*, CXX_*, AR_* for specific targets
    let target_upper = target.replace("-", "_").to_uppercase();
    let cc_env_var = format!("CC_{}", target_upper);
    let cxx_env_var = format!("CXX_{}", target_upper);
    let ar_env_var = format!("AR_{}", target_upper);
    let linker_var_name = format!("CARGO_TARGET_{}_LINKER", target_upper);

    // Set target-specific compiler variables (cc-rs uses these)
    // Format: CC_x86_64_unknown_linux_gnu, CXX_x86_64_unknown_linux_gnu, etc.
    cmd.env(&cc_env_var, cc_wrapper_str);
    cmd.env(&cxx_env_var, cxx_wrapper_str);
    cmd.env(&ar_env_var, ar_wrapper_str);
    cmd.env(&linker_var_name, cc_wrapper_str);

    // Debug: Print what we're setting (only in verbose mode)
    if std::env::var("RUST_LOG").is_ok() || std::env::var("VERBOSE").is_ok() {
        eprintln!("  Setting {}={}", cc_env_var, cc_wrapper_str);
        eprintln!("  Setting {}={}", cxx_env_var, cxx_wrapper_str);
        eprintln!("  Setting {}={}", ar_env_var, ar_wrapper_str);
    }

    // For cross-compilation, we MUST unset CC/CXX to force cc-rs to use our CC_* variables
    // cc-rs will prefer CC/CXX over CC_* if both are set, so we need to remove them
    if target.contains("linux") || target.contains("windows") {
        // Always remove CC/CXX for cross-compilation to ensure cc-rs uses our CC_* variables
        cmd.env_remove("CC");
        cmd.env_remove("CXX");
        cmd.env_remove("AR");
        // Also remove CFLAGS/CXXFLAGS that might interfere
        cmd.env_remove("CFLAGS");
        cmd.env_remove("CXXFLAGS");
    }

    // Set RUSTFLAGS to handle FFI symbol conflicts
    // Always set this for gnu targets - matches Dockerfile behavior
    // Note: LLD doesn't support this flag, but we set it anyway to match Docker behavior
    // The real solution would be to fix the duplicate symbol at source level
    if target.contains("linux-gnu") {
        let existing_rustflags = std::env::var("RUSTFLAGS").unwrap_or_default();
        let rustflags = if existing_rustflags.is_empty() {
            "-C link-arg=-Wl,--allow-multiple-definition".to_string()
        } else {
            format!(
                "{} -C link-arg=-Wl,--allow-multiple-definition",
                existing_rustflags
            )
        };
        cmd.env("RUSTFLAGS", rustflags);
    }

    // Modify PATH to put our wrapper directory first, ensuring our wrappers are found
    // This helps prevent cc-rs from finding clang in PATH
    // Also filter out common clang/llvm paths to force use of our Zig wrapper
    let current_path = std::env::var("PATH").unwrap_or_default();
    let wrapper_dir = temp_dir.to_string_lossy().to_string();

    // Filter out common clang/llvm paths from PATH to prevent cc-rs from finding clang
    let filtered_path: Vec<&str> = current_path
        .split(':')
        .filter(|p| {
            !p.contains("/opt/homebrew/opt/llvm")
                && !p.contains("/usr/local/opt/llvm")
                && !p.contains("/Applications/Xcode.app/Contents/Developer/Toolchains")
                && !p.contains("/usr/bin") // Don't filter /usr/bin, but prioritize our wrapper
        })
        .collect();

    let new_path = if filtered_path.is_empty() {
        wrapper_dir
    } else {
        format!("{}:{}", wrapper_dir, filtered_path.join(":"))
    };
    cmd.env("PATH", &new_path);

    println!(
        "  ✓ Configured Zig for cross-compilation to {} (Zig target: {})",
        target, zig_target
    );
    Ok(())
}

/// Convert Rust target triple to Zig target format
fn rust_target_to_zig_target(target: &str) -> String {
    if target == "x86_64-pc-windows-msvc" {
        "x86_64-windows".to_string()
    } else if target == "aarch64-pc-windows-msvc" {
        "aarch64-windows".to_string()
    } else if target == "x86_64-unknown-linux-gnu" {
        "x86_64-linux-gnu".to_string()
    } else if target == "aarch64-unknown-linux-gnu" {
        "aarch64-linux-gnu".to_string()
    } else if target == "x86_64-unknown-linux-musl" {
        "x86_64-linux-musl".to_string()
    } else if target == "aarch64-unknown-linux-musl" {
        "aarch64-linux-musl".to_string()
    } else {
        // Fallback: try to convert Rust target to Zig format
        target
            .replace("unknown-linux-gnu", "linux-gnu")
            .replace("unknown-linux-musl", "linux-musl")
            .replace("pc-windows-msvc", "windows")
    }
}
