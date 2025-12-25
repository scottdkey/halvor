#!/bin/bash

# Migration script to reorganize halvor into monorepo structure
# Run this from the repository root

set -e

echo "Starting monorepo migration..."

# Create new directories
echo "Creating new directory structure..."
mkdir -p projects/core projects/android projects/ios projects/web projects/ffi-macro projects/vpn-container

# Move source code
echo "Moving src/ to projects/core/..."
if [ -d "src" ] && [ ! -f "projects/core/lib.rs" ]; then
    mv src/* projects/core/
    rmdir src
    echo "✓ Moved src/ to projects/core/"
else
    echo "⚠️  src/ directory not found or projects/core/ already has files (may already be moved)"
fi

# Move platform directories
echo "Moving halvor-android/ to projects/android/..."
if [ -d "halvor-android" ] && [ -z "$(ls -A projects/android 2>/dev/null)" ]; then
    mv halvor-android/* projects/android/
    rmdir halvor-android
    echo "✓ Moved halvor-android/ to projects/android/"
else
    echo "⚠️  halvor-android/ directory not found or projects/android/ already has files (may already be moved)"
fi

echo "Moving halvor-swift/ to projects/ios/..."
if [ -d "halvor-swift" ] && [ -z "$(ls -A projects/ios 2>/dev/null)" ]; then
    mv halvor-swift/* projects/ios/
    rmdir halvor-swift
    echo "✓ Moved halvor-swift/ to projects/ios/"
else
    echo "⚠️  halvor-swift/ directory not found or projects/ios/ already has files (may already be moved)"
fi

echo "Moving halvor-web/ to projects/web/..."
if [ -d "halvor-web" ] && [ -z "$(ls -A projects/web 2>/dev/null)" ]; then
    mv halvor-web/* projects/web/
    rmdir halvor-web
    echo "✓ Moved halvor-web/ to projects/web/"
else
    echo "⚠️  halvor-web/ directory not found or projects/web/ already has files (may already be moved)"
fi

# Move crates
echo "Moving crates/halvor-ffi-macro/ to projects/ffi-macro/..."
if [ -d "crates/halvor-ffi-macro" ] && [ -z "$(ls -A projects/ffi-macro 2>/dev/null)" ]; then
    mv crates/halvor-ffi-macro/* projects/ffi-macro/
    rmdir crates/halvor-ffi-macro
    if [ -d "crates" ] && [ -z "$(ls -A crates)" ]; then
        rmdir crates
    fi
    echo "✓ Moved crates/halvor-ffi-macro/ to projects/ffi-macro/"
else
    echo "⚠️  crates/halvor-ffi-macro/ directory not found or projects/ffi-macro/ already has files (may already be moved)"
fi

# Move container
echo "Moving openvpn-container/ to projects/vpn-container/..."
if [ -d "openvpn-container" ] && [ -z "$(ls -A projects/vpn-container 2>/dev/null)" ]; then
    mv openvpn-container/* projects/vpn-container/
    rmdir openvpn-container
    echo "✓ Moved openvpn-container/ to projects/vpn-container/"
else
    echo "⚠️  openvpn-container/ directory not found or projects/vpn-container/ already has files (may already be moved)"
fi

# Backup old Cargo.toml if it exists and isn't already a workspace
if [ -f "Cargo.toml" ] && ! grep -q "^\[workspace\]" Cargo.toml; then
    if [ ! -f "Cargo.toml.old" ]; then
        cp Cargo.toml Cargo.toml.old
        echo "✓ Backed up old Cargo.toml to Cargo.toml.old"
    fi
fi

# Create projects/core/Cargo.toml from old root Cargo.toml if it doesn't exist
echo "Setting up projects/core/Cargo.toml..."
if [ -f "Cargo.toml.old" ] && [ ! -f "projects/core/Cargo.toml" ]; then
    cp Cargo.toml.old projects/core/Cargo.toml
    # Update paths in projects/core/Cargo.toml
    sed -i.bak 's|path = "src/lib.rs"|path = "lib.rs"|g' projects/core/Cargo.toml
    sed -i.bak 's|path = "src/main.rs"|path = "main.rs"|g' projects/core/Cargo.toml
    sed -i.bak 's|path = "crates/halvor-ffi-macro"|path = "../ffi-macro"|g' projects/core/Cargo.toml
    rm -f projects/core/Cargo.toml.bak
    echo "✓ Created projects/core/Cargo.toml"
elif [ -f "projects/core/Cargo.toml" ]; then
    echo "✓ projects/core/Cargo.toml already exists"
else
    echo "⚠️  Could not create projects/core/Cargo.toml - Cargo.toml.old not found"
fi

# Verify workspace Cargo.toml exists
if [ ! -f "Cargo.toml" ] || ! grep -q "^\[workspace\]" Cargo.toml; then
    echo "⚠️  Warning: Root Cargo.toml is not a workspace file"
    echo "   The workspace Cargo.toml should have been created already"
    echo "   Please check that Cargo.toml contains [workspace] section"
fi

echo ""
echo "✓ Migration complete!"
echo ""
echo "Next steps:"
echo "1. Verify workspace builds: cargo check --workspace"
echo "2. Test CLI build: cargo build --release --bin halvor --manifest-path projects/core/Cargo.toml"
echo "3. Review MONOREPO_STATUS.md for remaining updates"
echo "4. Update any remaining path references in:"
echo "   - Build scripts (projects/ios/build.sh, projects/vpn-container/build.sh)"
echo "   - Dockerfiles (projects/web/Dockerfile)"
echo "   - CI/CD workflows (.github/workflows/*.yml)"
echo "   - Fastlane files (fastlane/Fastfile)"
