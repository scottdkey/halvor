// Android build and signing
use crate::services::build::common::{copy_file, ensure_dir_exists, execute_command};
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Command;

/// Android target triplets and their corresponding Android ABI directories
const ANDROID_TARGETS: &[(&str, &str)] = &[
    ("aarch64-linux-android", "arm64-v8a"),
    ("armv7-linux-androideabi", "armeabi-v7a"),
    ("i686-linux-android", "x86"),
    ("x86_64-linux-android", "x86_64"),
];

/// Build Android JNI library for all targets
pub fn build_android() -> Result<()> {
    println!("Building Android JNI library...");

    // Build for all Android targets
    for (target, _) in ANDROID_TARGETS {
        println!("Building for target: {}", target);
        let status = Command::new("cargo")
            .args(["build", "--lib", "--release", "--target", target])
            .status()
            .context(format!("Failed to build for {}", target))?;

        if !status.success() {
            anyhow::bail!("Failed to build for {}", target);
        }
    }

    println!("Copying JNI libraries to Android project...");
    let jni_libs = PathBuf::from("halvor-android/src/main/jniLibs");

    // Create directories and copy libraries
    for (target, arch) in ANDROID_TARGETS {
        let lib_dir = jni_libs.join(arch);
        ensure_dir_exists(&lib_dir)?;

        let src_lib = PathBuf::from("target")
            .join(target)
            .join("release")
            .join("libhalvor.so");

        let dst_lib = lib_dir.join("libhalvor_jni.so");

        copy_file(&src_lib, &dst_lib)?;
    }

    println!("Building Android app...");
    let gradle_dir = PathBuf::from("halvor-android");
    let mut gradle_cmd = Command::new("./gradlew");
    gradle_cmd.arg("build").current_dir(&gradle_dir);

    execute_command(gradle_cmd, "Android Gradle build failed")?;

    Ok(())
}

/// Sign Android app using Gradle
pub fn sign_android() -> Result<()> {
    println!("Signing Android app...");
    let gradle_dir = PathBuf::from("halvor-android");
    let mut gradle_cmd = Command::new("./gradlew");
    gradle_cmd
        .args(["assembleRelease", "bundleRelease"])
        .current_dir(&gradle_dir);

    execute_command(gradle_cmd, "Android signing failed")?;

    Ok(())
}
