# Monorepo Migration Guide

This document outlines the steps to reorganize halvor into a monorepo structure.

## New Structure

```
halvor/
├── projects/
│   ├── core/          # src/ → projects/core/ (main halvor CLI and library)
│   ├── android/       # halvor-android/ → projects/android/
│   ├── ios/           # halvor-swift/ → projects/ios/
│   ├── web/           # halvor-web/ → projects/web/
│   ├── ffi-macro/     # crates/halvor-ffi-macro/ → projects/ffi-macro/
│   └── vpn-container/ # openvpn-container/ → projects/vpn-container/
├── charts/            # Keep as-is
├── fastlane/          # Keep as-is
├── scripts/           # Keep as-is
├── docs/              # Keep as-is
└── Cargo.toml         # Workspace root
```

## Migration Steps

### 1. Move Directories

```bash
# Create new directories
mkdir -p projects/core projects/android projects/ios projects/web projects/ffi-macro projects/vpn-container

# Move source code
mv src/* projects/core/
rmdir src

# Move platform directories
mv halvor-android/* projects/android/
rmdir halvor-android

mv halvor-swift/* projects/ios/
rmdir halvor-swift

mv halvor-web/* projects/web/
rmdir halvor-web

# Move crates
mv crates/halvor-ffi-macro/* projects/ffi-macro/
rmdir crates/halvor-ffi-macro
rmdir crates

# Move container
mv openvpn-container/* projects/vpn-container/
rmdir openvpn-container
```

### 2. Update Cargo.toml Files

- Root `Cargo.toml` → Convert to workspace with `projects/` paths
- `projects/core/Cargo.toml` → Update from current root Cargo.toml, change path references
- `projects/ffi-macro/Cargo.toml` → Update path references
- `projects/vpn-container/Cargo.toml` → Update path references

### 3. Update Code References

Search and replace:
- `crates/halvor-ffi-macro` → `projects/ffi-macro`
- `halvor-android` → `projects/android`
- `halvor-swift` → `projects/ios`
- `halvor-web` → `projects/web`
- `openvpn-container` → `projects/vpn-container`
- `src/` → `projects/core/` (in build scripts, docs, etc.)

### 4. Update Makefile

- Update all paths to new directory structure
- Update build commands
- Update install commands

### 5. Update Documentation

- Update all path references in docs/
- Update README.md
- Update CLAUDE.md

### 6. Optional: Remove Unused Directories

- `compose/` - Check if still needed (legacy Docker Compose files)
- `cluster/` - Check if still needed (K3s config files)
- `releases/` - Check if still needed (Helm release values)

## Files to Update

### Cargo.toml Files
- Root: Convert to workspace
- core/Cargo.toml: Main application
- ffi-macro/Cargo.toml: FFI macro crate
- vpn-container/Cargo.toml: VPN container

### Build Scripts
- Makefile
- scripts/generate-docs.sh
- projects/ios/build.sh
- projects/vpn-container/build.sh

### Code Files
- projects/core/src/services/docker/mod.rs (openvpn-container → projects/vpn-container)
- projects/core/src/services/pia_vpn/deploy.rs (compose/ paths)
- projects/core/src/services/build/*.rs (all platform paths)
- projects/core/src/services/dev/*.rs (all platform paths)

### Documentation
- docs/development.md
- docs/architecture.md
- README.md
- CLAUDE.md

### CI/CD
- .github/workflows/*.yml (all workflow files)

