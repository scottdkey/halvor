#!/bin/bash
set -e

# Build script for Halvor Swift bridge
# This script builds the Rust FFI library for iOS and macOS
# Usage: ./build.sh [macos|ios]
# If no argument provided, builds for macOS

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
OUTPUT_DIR="$SCRIPT_DIR/Sources/HalvorSwiftFFI"

# Parse platform argument
PLATFORM="${1:-macos}"
if [ "$PLATFORM" != "macos" ] && [ "$PLATFORM" != "ios" ]; then
    echo "Error: Platform must be 'macos' or 'ios'"
    exit 1
fi

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Building Halvor Swift bridge for ${PLATFORM}...${NC}"

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: cargo is not installed. Please install Rust first.${NC}"
    exit 1
fi

# Check if we can generate bindings
# Note: We use manual C FFI, not UniFFI

# Create output directory
mkdir -p "$OUTPUT_DIR"

cd "$ROOT_DIR"

# Clean previous builds for the target platform
echo -e "${YELLOW}Cleaning previous builds...${NC}"

# Build based on platform
if [ "$PLATFORM" = "macos" ]; then
    # Build the library for macOS from main crate
    echo -e "${YELLOW}Building library for macOS...${NC}"
    export MACOSX_DEPLOYMENT_TARGET=15.0
    cargo build --lib --release --target aarch64-apple-darwin 2>&1 | tail -10 || {
        echo -e "${RED}Failed to build library for macOS${NC}"
        exit 1
    }
    
    # Detect host architecture
    HOST_ARCH=$(uname -m)
    echo -e "${YELLOW}Host architecture: ${HOST_ARCH}${NC}"
    
    # Build x86_64 if on Intel Mac
    if [ "$HOST_ARCH" = "x86_64" ]; then
        echo -e "${YELLOW}Building for x86_64 macOS...${NC}"
        export MACOSX_DEPLOYMENT_TARGET=15.0
        cargo build --lib --release --target x86_64-apple-darwin 2>&1 | tail -10 || {
            echo -e "${YELLOW}Warning: Failed to build for x86_64, continuing with arm64 only${NC}"
        }
    fi
elif [ "$PLATFORM" = "ios" ]; then
    # Build the library for iOS from main crate
    echo -e "${YELLOW}Building library for iOS...${NC}"
    
    # Install iOS targets if needed
    if ! rustup target list --installed | grep -q "aarch64-apple-ios"; then
        echo -e "${YELLOW}Installing iOS targets...${NC}"
        rustup target add aarch64-apple-ios || true
        rustup target add aarch64-apple-ios-sim || true
    fi
    
    export MACOSX_DEPLOYMENT_TARGET=15.0
    cargo build --lib --release --target aarch64-apple-ios 2>&1 | tail -10 || {
        echo -e "${RED}Failed to build library for iOS${NC}"
        exit 1
    }
    
    echo -e "${YELLOW}Building library for iOS Simulator (arm64)...${NC}"
    export MACOSX_DEPLOYMENT_TARGET=15.0
    cargo build --lib --release --target aarch64-apple-ios-sim 2>&1 | tail -10 || {
        echo -e "${RED}Failed to build library for iOS Simulator (arm64)${NC}"
        exit 1
    }
    
    # Detect host architecture and build x86_64 simulator if on Intel Mac
    HOST_ARCH=$(uname -m)
    if [ "$HOST_ARCH" = "x86_64" ]; then
        echo -e "${YELLOW}Building library for iOS Simulator (x86_64)...${NC}"
        if ! rustup target list --installed | grep -q "x86_64-apple-ios"; then
            rustup target add x86_64-apple-ios 2>/dev/null || true
        fi
        export MACOSX_DEPLOYMENT_TARGET=15.0
        cargo build --lib --release --target x86_64-apple-ios 2>&1 | tail -10 || {
            echo -e "${YELLOW}Warning: Failed to build for x86_64 iOS Simulator, continuing with arm64 only${NC}"
        }
    fi
fi

cd "$SCRIPT_DIR"

# Create XCFramework
echo -e "${YELLOW}Creating XCFramework for ${PLATFORM}...${NC}"

FRAMEWORK_DIR="$OUTPUT_DIR/HalvorSwiftFFI.xcframework"
# Remove existing XCFramework completely to ensure clean build for this platform
# This prevents mixing iOS and macOS slices
echo -e "${YELLOW}Cleaning existing XCFramework...${NC}"
rm -rf "$FRAMEWORK_DIR"
# Also clean any leftover framework directories
rm -rf "$OUTPUT_DIR/macos-arm64" "$OUTPUT_DIR/macos-x86_64" "$OUTPUT_DIR/ios-arm64" "$OUTPUT_DIR/ios-arm64-simulator" "$OUTPUT_DIR/ios-x86_64-simulator" 2>/dev/null || true

# Create framework structure for each platform
create_framework() {
    local platform=$1
    local arch=$2
    local target=$3
    local sdk=$4
    
    local framework_name="HalvorSwiftFFI"
    local framework_path="$FRAMEWORK_DIR/$platform/$framework_name.framework"
    
    # macOS requires Versions structure, iOS uses flat structure
    if [ "$platform" == "macos" ] || [[ "$platform" == macos-* ]]; then
        mkdir -p "$framework_path/Versions/A/Headers"
        mkdir -p "$framework_path/Versions/A/Modules"
        mkdir -p "$framework_path/Versions/A/Resources"
    else
        mkdir -p "$framework_path/Headers"
        mkdir -p "$framework_path/Modules"
    fi
    
    # Copy library (from main crate)
    # Try .a first (staticlib), then .dylib (cdylib)
    local lib_path="$ROOT_DIR/target/$target/release/libhalvor.a"
    if [ ! -f "$lib_path" ]; then
        lib_path="$ROOT_DIR/target/$target/release/libhalvor.dylib"
        if [ ! -f "$lib_path" ]; then
            echo -e "${RED}Error: Library not found at $lib_path${NC}"
            echo -e "${YELLOW}Looking for: $ROOT_DIR/target/$target/release/libhalvor.{a,dylib}${NC}"
            return 1
        fi
        # For iOS, we need to convert dylib to static lib or use it directly
        echo -e "${YELLOW}Using .dylib instead of .a for $target${NC}"
    fi
    
    # Use lipo to create universal binary if needed, or just copy
    if [ "$platform" == "macos" ] || [[ "$platform" == macos-* ]]; then
        # For macOS, use Versions structure
        if [ "$arch" == "arm64" ]; then
            # This will be combined later
            cp "$lib_path" "$framework_path/Versions/A/HalvorSwiftFFI-arm64.a"
        else
            cp "$lib_path" "$framework_path/Versions/A/HalvorSwiftFFI-x86_64.a"
        fi
    else
        # For iOS, use flat structure
        cp "$lib_path" "$framework_path/HalvorSwiftFFI"
    fi
    
    # Determine paths based on platform
    if [ "$platform" == "macos" ] || [[ "$platform" == macos-* ]]; then
        HEADERS_DIR="$framework_path/Versions/A/Headers"
        MODULES_DIR="$framework_path/Versions/A/Modules"
        RESOURCES_DIR="$framework_path/Versions/A/Resources"
    else
        HEADERS_DIR="$framework_path/Headers"
        MODULES_DIR="$framework_path/Modules"
    fi
    
    # Copy generated Swift bindings (they're platform-agnostic)
    if [ -f "$OUTPUT_DIR/HalvorSwiftFFI.swift" ]; then
        if [ "$platform" == "macos" ] || [[ "$platform" == macos-* ]]; then
            mkdir -p "$RESOURCES_DIR"
            cp "$OUTPUT_DIR/HalvorSwiftFFI.swift" "$RESOURCES_DIR/"
        else
            mkdir -p "$framework_path/Swift"
            cp "$OUTPUT_DIR/HalvorSwiftFFI.swift" "$framework_path/Swift/"
        fi
    fi
    
    # Copy headers
    if [ -f "$OUTPUT_DIR/halvor_ffi.h" ]; then
        cp "$OUTPUT_DIR/halvor_ffi.h" "$HEADERS_DIR/"
    fi
    
    # Create module map
    if [ -f "$OUTPUT_DIR/halvor_ffi.h" ]; then
        cat > "$MODULES_DIR/module.modulemap" <<EOF
framework module HalvorSwiftFFI {
    umbrella header "halvor_ffi.h"
    export *
    module * { export * }
}
EOF
    else
        cat > "$MODULES_DIR/module.modulemap" <<EOF
framework module HalvorSwiftFFI {
    export *
}
EOF
    fi
    
    # Create Info.plist
    if [ "$platform" == "macos" ] || [[ "$platform" == macos-* ]]; then
        INFO_PLIST="$RESOURCES_DIR/Info.plist"
    else
        INFO_PLIST="$framework_path/Info.plist"
    fi
    cat > "$INFO_PLIST" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>HalvorSwiftFFI</string>
    <key>CFBundleIdentifier</key>
    <string>dev.scottkey.halvor.ffi</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundlePackageType</key>
    <string>FMWK</string>
    <key>CFBundleShortVersionString</key>
    <string>0.0.6</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>MinimumOSVersion</key>
    <string>13.0</string>
</dict>
</plist>
EOF
    
    # Note: Symlinks for macOS will be created after the binary is finalized (after universal binary creation)
}

# Create frameworks for the specified platform
if [ "$PLATFORM" = "macos" ]; then
    create_framework "macos-arm64" "arm64" "aarch64-apple-darwin" "macosx"
    HOST_ARCH=$(uname -m)
    if [ "$HOST_ARCH" = "x86_64" ]; then
        create_framework "macos-x86_64" "x86_64" "x86_64-apple-darwin" "macosx"
    fi
elif [ "$PLATFORM" = "ios" ]; then
    create_framework "ios-arm64" "arm64" "aarch64-apple-ios" "iphoneos"
    create_framework "ios-arm64-simulator" "arm64" "aarch64-apple-ios-sim" "iphonesimulator"
    # Build x86_64 simulator if on Intel Mac (for Rosetta compatibility)
    HOST_ARCH=$(uname -m)
    if [ "$HOST_ARCH" = "x86_64" ] && rustup target list --installed | grep -q "x86_64-apple-ios"; then
        if [ -f "$ROOT_DIR/target/x86_64-apple-ios/release/libhalvor.a" ]; then
            create_framework "ios-x86_64-simulator" "x86_64" "x86_64-apple-ios" "iphonesimulator"
        fi
    fi
fi

# Combine architectures into universal binary (only for macOS, only if both exist)
if [ "$PLATFORM" = "macos" ]; then
    HOST_ARCH=$(uname -m)
    if [ "$HOST_ARCH" = "x86_64" ] && \
       [ -f "$FRAMEWORK_DIR/macos-arm64/HalvorSwiftFFI.framework/Versions/A/HalvorSwiftFFI-arm64.a" ] && \
       [ -f "$FRAMEWORK_DIR/macos-x86_64/HalvorSwiftFFI.framework/Versions/A/HalvorSwiftFFI-x86_64.a" ]; then
        echo -e "${YELLOW}Creating universal macOS binary...${NC}"
        framework_path="$FRAMEWORK_DIR/macos-arm64/HalvorSwiftFFI.framework"
        lipo -create \
            "$framework_path/Versions/A/HalvorSwiftFFI-arm64.a" \
            "$FRAMEWORK_DIR/macos-x86_64/HalvorSwiftFFI.framework/Versions/A/HalvorSwiftFFI-x86_64.a" \
            -output "$framework_path/Versions/A/HalvorSwiftFFI"
        rm "$framework_path/Versions/A/HalvorSwiftFFI-arm64.a"
        rm "$FRAMEWORK_DIR/macos-x86_64/HalvorSwiftFFI.framework/Versions/A/HalvorSwiftFFI-x86_64.a"
        # Make the binary executable
        chmod +x "$framework_path/Versions/A/HalvorSwiftFFI"
        # Create symlinks for macOS framework structure (after binary is in place)
        # Versions/Current should point to A (relative to Versions directory)
        (cd "$framework_path/Versions" && ln -sfh A Current) && \
        (cd "$framework_path" && \
         ln -sfh Versions/Current/Headers Headers && \
         ln -sfh Versions/Current/Modules Modules && \
         ln -sfh Versions/Current/Resources Resources && \
         ln -sfh Versions/Current/HalvorSwiftFFI HalvorSwiftFFI)
    elif [ "$HOST_ARCH" = "arm64" ]; then
        # On ARM Mac, just rename the arm64 library
        framework_path="$FRAMEWORK_DIR/macos-arm64/HalvorSwiftFFI.framework"
        if [ -f "$framework_path/Versions/A/HalvorSwiftFFI-arm64.a" ]; then
            mv "$framework_path/Versions/A/HalvorSwiftFFI-arm64.a" \
               "$framework_path/Versions/A/HalvorSwiftFFI"
            # Make the binary executable
            chmod +x "$framework_path/Versions/A/HalvorSwiftFFI"
        fi
        # Create symlinks for macOS framework structure (after binary is in place)
        # Versions/Current should point to A (relative to Versions directory)
        (cd "$framework_path/Versions" && ln -sfh A Current) && \
        (cd "$framework_path" && \
         ln -sfh Versions/Current/Headers Headers && \
         ln -sfh Versions/Current/Modules Modules && \
         ln -sfh Versions/Current/Resources Resources && \
         ln -sfh Versions/Current/HalvorSwiftFFI HalvorSwiftFFI)
    fi
fi

# Create XCFramework using xcodebuild
echo -e "${YELLOW}Assembling XCFramework...${NC}"
XCFRAMEWORK_ARGS=()
if [ "$PLATFORM" = "macos" ]; then
    XCFRAMEWORK_ARGS+=(-framework "$FRAMEWORK_DIR/macos-arm64/HalvorSwiftFFI.framework")
    HOST_ARCH=$(uname -m)
    if [ "$HOST_ARCH" = "x86_64" ] && [ -d "$FRAMEWORK_DIR/macos-x86_64" ]; then
        XCFRAMEWORK_ARGS+=(-framework "$FRAMEWORK_DIR/macos-x86_64/HalvorSwiftFFI.framework")
    fi
elif [ "$PLATFORM" = "ios" ]; then
    XCFRAMEWORK_ARGS+=(-framework "$FRAMEWORK_DIR/ios-arm64/HalvorSwiftFFI.framework")
    XCFRAMEWORK_ARGS+=(-framework "$FRAMEWORK_DIR/ios-arm64-simulator/HalvorSwiftFFI.framework")
    if [ -d "$FRAMEWORK_DIR/ios-x86_64-simulator" ]; then
        XCFRAMEWORK_ARGS+=(-framework "$FRAMEWORK_DIR/ios-x86_64-simulator/HalvorSwiftFFI.framework")
    fi
fi
XCFRAMEWORK_ARGS+=(-output "$FRAMEWORK_DIR")

xcodebuild -create-xcframework "${XCFRAMEWORK_ARGS[@]}" 2>/dev/null || {
    # Fallback: manually create XCFramework structure
    echo -e "${YELLOW}Using manual XCFramework creation...${NC}"
    # Create temporary directory for frameworks
    TEMP_FRAMEWORK_DIR="$OUTPUT_DIR/temp_frameworks"
    mkdir -p "$TEMP_FRAMEWORK_DIR"
    
    # Only move frameworks for the current platform
    if [ "$PLATFORM" = "macos" ]; then
        mv "$FRAMEWORK_DIR/macos-arm64" "$TEMP_FRAMEWORK_DIR/" 2>/dev/null || true
        HOST_ARCH=$(uname -m)
        if [ "$HOST_ARCH" = "x86_64" ] && [ -d "$FRAMEWORK_DIR/macos-x86_64" ]; then
            mv "$FRAMEWORK_DIR/macos-x86_64" "$TEMP_FRAMEWORK_DIR/" 2>/dev/null || true
        fi
    elif [ "$PLATFORM" = "ios" ]; then
        mv "$FRAMEWORK_DIR/ios-arm64" "$TEMP_FRAMEWORK_DIR/" 2>/dev/null || true
        mv "$FRAMEWORK_DIR/ios-arm64-simulator" "$TEMP_FRAMEWORK_DIR/" 2>/dev/null || true
        mv "$FRAMEWORK_DIR/ios-x86_64-simulator" "$TEMP_FRAMEWORK_DIR/" 2>/dev/null || true
    fi
    
    # Now move to final location
    rm -rf "$FRAMEWORK_DIR"
    mkdir -p "$FRAMEWORK_DIR"
    mv "$TEMP_FRAMEWORK_DIR"/* "$FRAMEWORK_DIR/" 2>/dev/null || true
    rmdir "$TEMP_FRAMEWORK_DIR" 2>/dev/null || true
    
    # Create Info.plist for XCFramework (platform-specific)
    if [ "$PLATFORM" = "macos" ]; then
        HOST_ARCH=$(uname -m)
        cat > "$FRAMEWORK_DIR/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>AvailableLibraries</key>
    <array>
        <dict>
            <key>LibraryIdentifier</key>
            <string>macos-arm64</string>
            <key>LibraryPath</key>
            <string>HalvorSwiftFFI.framework</string>
            <key>SupportedArchitectures</key>
            <array>
                <string>arm64</string>
EOF
        if [ "$HOST_ARCH" = "x86_64" ] && [ -d "$FRAMEWORK_DIR/macos-x86_64" ]; then
            cat >> "$FRAMEWORK_DIR/Info.plist" <<EOF
                <string>x86_64</string>
EOF
        fi
        cat >> "$FRAMEWORK_DIR/Info.plist" <<EOF
            </array>
            <key>SupportedPlatform</key>
            <string>macos</string>
        </dict>
EOF
        if [ "$HOST_ARCH" = "x86_64" ] && [ -d "$FRAMEWORK_DIR/macos-x86_64" ]; then
            cat >> "$FRAMEWORK_DIR/Info.plist" <<EOF
        <dict>
            <key>LibraryIdentifier</key>
            <string>macos-x86_64</string>
            <key>LibraryPath</key>
            <string>HalvorSwiftFFI.framework</string>
            <key>SupportedArchitectures</key>
            <array>
                <string>x86_64</string>
            </array>
            <key>SupportedPlatform</key>
            <string>macos</string>
        </dict>
EOF
        fi
        cat >> "$FRAMEWORK_DIR/Info.plist" <<EOF
    </array>
    <key>CFBundlePackageType</key>
    <string>XFWK</string>
    <key>XCFrameworkFormatVersion</key>
    <string>1.0</string>
</dict>
</plist>
EOF
    elif [ "$PLATFORM" = "ios" ]; then
        cat > "$FRAMEWORK_DIR/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>AvailableLibraries</key>
    <array>
        <dict>
            <key>LibraryIdentifier</key>
            <string>ios-arm64</string>
            <key>LibraryPath</key>
            <string>HalvorSwiftFFI.framework</string>
            <key>SupportedArchitectures</key>
            <array>
                <string>arm64</string>
            </array>
            <key>SupportedPlatform</key>
            <string>ios</string>
        </dict>
        <dict>
            <key>LibraryIdentifier</key>
            <string>ios-arm64-simulator</string>
            <key>LibraryPath</key>
            <string>HalvorSwiftFFI.framework</string>
            <key>SupportedArchitectures</key>
            <array>
                <string>arm64</string>
            </array>
            <key>SupportedPlatform</key>
            <string>ios</string>
            <key>SupportedPlatformVariant</key>
            <string>simulator</string>
        </dict>
    </array>
    <key>CFBundlePackageType</key>
    <string>XFWK</string>
    <key>XCFrameworkFormatVersion</key>
    <string>1.0</string>
</dict>
</plist>
EOF
    fi
}

# Clean up - ensure only the current platform's frameworks remain
if [ "$PLATFORM" = "macos" ]; then
    # Remove any iOS frameworks that might have been left over
    rm -rf "$FRAMEWORK_DIR/ios-arm64" 2>/dev/null || true
    rm -rf "$FRAMEWORK_DIR/ios-arm64-simulator" 2>/dev/null || true
    rm -rf "$FRAMEWORK_DIR/ios-x86_64-simulator" 2>/dev/null || true
    HOST_ARCH=$(uname -m)
    if [ "$HOST_ARCH" = "arm64" ] && [ -d "$FRAMEWORK_DIR/macos-x86_64" ]; then
        rm -rf "$FRAMEWORK_DIR/macos-x86_64"
    fi
elif [ "$PLATFORM" = "ios" ]; then
    # Remove any macOS frameworks that might have been left over
    rm -rf "$FRAMEWORK_DIR/macos-arm64" 2>/dev/null || true
    rm -rf "$FRAMEWORK_DIR/macos-x86_64" 2>/dev/null || true
fi

echo -e "${GREEN}Build complete! XCFramework created at: $FRAMEWORK_DIR${NC}"
