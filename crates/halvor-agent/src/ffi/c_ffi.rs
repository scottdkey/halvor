// C FFI bindings for Swift
// This module exports C-compatible functions that Swift can call

use crate::agent::client::HalvorClient;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

/// Opaque pointer type for HalvorClient (matches C typedef)
pub type HalvorClientPtr = *mut HalvorClient;

/// Create a new Halvor client
/// Returns a pointer to the client, or NULL on error
/// agent_port: 0 means use default port
#[unsafe(no_mangle)]
pub unsafe extern "C" fn halvor_client_new(agent_port: u16) -> HalvorClientPtr {
    let port = if agent_port == 0 {
        None
    } else {
        Some(agent_port)
    };
    let client = HalvorClient::new(port);
    Box::into_raw(Box::new(client))
}

/// Free a Halvor client
#[unsafe(no_mangle)]
pub unsafe extern "C" fn halvor_client_free(ptr: HalvorClientPtr) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Discover all agents
/// Returns JSON string with array of DiscoveredHost, or NULL on error
/// Caller must free the returned string with halvor_string_free
#[unsafe(no_mangle)]
pub unsafe extern "C" fn halvor_client_discover_agents(ptr: HalvorClientPtr) -> *mut c_char {
    if ptr.is_null() {
        return ptr::null_mut();
    }

    let client = &*ptr;
    match client.discover_agents() {
        Ok(hosts) => match serde_json::to_string(&hosts) {
            Ok(json) => match CString::new(json) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => ptr::null_mut(),
            },
            Err(_) => ptr::null_mut(),
        },
        Err(_) => ptr::null_mut(),
    }
}

/// Discover agents via Tailscale
#[unsafe(no_mangle)]
pub unsafe extern "C" fn halvor_client_discover_via_tailscale(ptr: HalvorClientPtr) -> *mut c_char {
    if ptr.is_null() {
        return ptr::null_mut();
    }

    let client = &*ptr;
    match client.discover_via_tailscale() {
        Ok(hosts) => match serde_json::to_string(&hosts) {
            Ok(json) => match CString::new(json) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => ptr::null_mut(),
            },
            Err(_) => ptr::null_mut(),
        },
        Err(_) => ptr::null_mut(),
    }
}

/// Discover agents on local network
#[unsafe(no_mangle)]
pub unsafe extern "C" fn halvor_client_discover_via_local_network(
    ptr: HalvorClientPtr,
) -> *mut c_char {
    if ptr.is_null() {
        return ptr::null_mut();
    }

    let client = &*ptr;
    match client.discover_via_local_network() {
        Ok(hosts) => match serde_json::to_string(&hosts) {
            Ok(json) => match CString::new(json) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => ptr::null_mut(),
            },
            Err(_) => ptr::null_mut(),
        },
        Err(_) => ptr::null_mut(),
    }
}

/// Ping an agent
/// Returns 1 if reachable, 0 if not reachable or on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn halvor_client_ping_agent(
    ptr: HalvorClientPtr,
    host: *const c_char,
    port: u16,
) -> i32 {
    if ptr.is_null() || host.is_null() {
        return 0;
    }

    let host_str = match CStr::from_ptr(host).to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return 0,
    };

    let client = &*ptr;
    match client.ping_agent(host_str, port) {
        Ok(reachable) => {
            if reachable {
                1
            } else {
                0
            }
        }
        Err(_) => 0,
    }
}

/// Get host info
/// Returns JSON string with HostInfo, or NULL on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn halvor_client_get_host_info(
    ptr: HalvorClientPtr,
    host: *const c_char,
    port: u16,
) -> *mut c_char {
    if ptr.is_null() || host.is_null() {
        return ptr::null_mut();
    }

    let host_str = match CStr::from_ptr(host).to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return ptr::null_mut(),
    };

    let client = &*ptr;
    match client.get_host_info(host_str, port) {
        Ok(info) => match serde_json::to_string(&info) {
            Ok(json) => match CString::new(json) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => ptr::null_mut(),
            },
            Err(_) => ptr::null_mut(),
        },
        Err(_) => ptr::null_mut(),
    }
}

/// Execute a command
/// Returns JSON string with command output, or NULL on error
/// args_json: JSON array of strings, or NULL for empty array
#[unsafe(no_mangle)]
pub unsafe extern "C" fn halvor_client_execute_command(
    ptr: HalvorClientPtr,
    host: *const c_char,
    port: u16,
    command: *const c_char,
    args_json: *const c_char,
) -> *mut c_char {
    if ptr.is_null() || host.is_null() || command.is_null() {
        return ptr::null_mut();
    }

    let host_str = match CStr::from_ptr(host).to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return ptr::null_mut(),
    };

    let command_str = match CStr::from_ptr(command).to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return ptr::null_mut(),
    };

    let args: Vec<String> = if args_json.is_null() {
        Vec::new()
    } else {
        match CStr::from_ptr(args_json).to_str() {
            Ok(json) => match serde_json::from_str::<Vec<String>>(json) {
                Ok(v) => v,
                Err(_) => Vec::new(),
            },
            Err(_) => Vec::new(),
        }
    };

    let client = &*ptr;
    match client.execute_command(host_str, port, command_str, args) {
        Ok(output) => match CString::new(output) {
            Ok(c_str) => c_str.into_raw(),
            Err(_) => ptr::null_mut(),
        },
        Err(_) => ptr::null_mut(),
    }
}

/// Free a string returned by the FFI
#[unsafe(no_mangle)]
pub unsafe extern "C" fn halvor_string_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        let _ = CString::from_raw(ptr);
    }
}

