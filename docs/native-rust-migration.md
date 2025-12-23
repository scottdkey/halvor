# Native Rust Library Migration Plan

This document identifies CLI commands that can be replaced with native Rust libraries for better reliability, performance, and cross-platform compatibility.

## High Priority Replacements

### 1. Token Generation (`src/services/k3s/mod.rs`)

**Current**: Uses multiple CLI fallbacks (openssl, head+xxd, od, python3, shuf)
**Replace with**: `rand` crate (already in dependencies)

```rust
// Current: exec.execute_simple("openssl", &["rand", "-hex", "32"])
// Replace with:
use rand::Rng;
let mut rng = rand::thread_rng();
let token: String = (0..64).map(|_| format!("{:x}", rng.gen::<u8>())).collect();
```

**Files**: `src/services/k3s/mod.rs:14-186`

### 2. HTTP Downloads

**Current**: Multiple `curl` commands throughout codebase
**Replace with**: `reqwest` crate (already in dependencies, already used for K3s install script)

**Files to update**:
- `src/services/tailscale.rs:125` - Tailscale install script download
- `src/services/compose_deployer.rs:398,409` - Docker compose file downloads
- `src/services/pia_vpn/verify.rs:135,153` - IP checking (can use reqwest directly)
- `src/services/docker/mod.rs:319` - Docker GPG key download

### 3. Base64 Operations

**Current**: Some places still use `base64` CLI command
**Replace with**: `base64` crate (already imported in some files)

**Files to update**:
- `src/utils/ssh.rs:643` - SSH key checking uses `base64 -d`
- `src/services/sync.rs:82` - Database sync uses `base64 -d`

**Note**: `src/utils/crypto.rs` already uses the base64 crate correctly.

### 4. String Manipulation (grep/cut/tr)

**Current**: Uses shell commands for text processing
**Replace with**: Rust string methods or `regex` crate

**Files to update**:
- `src/utils/ssh.rs:544` - Password checking: `grep | cut | grep`
  - Can read `/etc/shadow` and parse directly
- `src/utils/ssh.rs:604` - Home directory extraction: `getent passwd | cut -d: -f6`
  - Can use `nix::unistd::User::from_name()` or parse directly
- `src/services/k3s/mod.rs:386,707,715,724` - Service checking: `grep k3s`
  - Can use string `contains()` or regex
- `src/services/pia_vpn/verify.rs:199` - Log parsing: `cat | tail | grep | tail`
  - Can read file, take last N lines, filter with regex

### 5. File Operations (head/tail/cat)

**Current**: Uses shell commands for file reading
**Replace with**: Rust file I/O

**Files to update**:
- `src/services/pia_vpn/verify.rs:199` - `tail -50` can use file reading with line limits
- `src/services/k3s/mod.rs:930` - `cat /etc/rancher/k3s/k3s.yaml` can use `read_file()`
- `src/services/k3s/mod.rs:915` - `head -5` can use iterator `.take(5)`

## Medium Priority Replacements

### 6. System Information

**Current**: Uses `hostname -I | awk '{print $1}'`
**Replace with**: Network interface enumeration

**Files**: `src/services/k3s/mod.rs:938`
- Can use `if_addrs` crate or `nix` for network interface info

### 7. Process/Service Checking

**Current**: Uses `pgrep`, `systemctl`, `docker exec`
**Replace with**: Native APIs where possible

**Note**: Some of these (systemctl, docker exec) may need to stay as CLI commands since they require system-level access. However:
- Process checking can use `nix::sys::signal` or process enumeration
- Service status can be checked via systemd D-Bus API (using `zbus` crate)

## Low Priority / Keep as CLI

### Commands that should remain CLI:
- `docker` - Docker CLI is the standard interface
- `kubectl` - Kubernetes CLI is the standard interface  
- `helm` - Helm CLI is the standard interface
- `systemctl` - System service management (though D-Bus API is possible)
- `sudo` - Required for privilege escalation
- `ssh` - Already using SSH libraries appropriately
- `chmod`, `chown`, `mkdir` - File permissions (can use `std::fs` but CLI is simpler for remote)

## Implementation Priority

1. **Token generation** - High impact, simple change, already has dependency
2. **HTTP downloads** - High impact, already partially done, consistent pattern
3. **Base64 operations** - Medium impact, already has dependency
4. **String manipulation** - Medium impact, improves reliability
5. **File operations** - Low impact, but cleaner code

## Dependencies to Add (if needed)

- `if_addrs` - For network interface enumeration
- `zbus` - For systemd D-Bus API (optional, systemctl is fine)
- `regex` - For advanced pattern matching (if not already used)

## Benefits

1. **Reliability**: No shell escaping issues
2. **Performance**: Native code is faster
3. **Cross-platform**: Works on Windows without WSL
4. **Error handling**: Better error messages and types
5. **Testing**: Easier to unit test native code
6. **Security**: No command injection risks
