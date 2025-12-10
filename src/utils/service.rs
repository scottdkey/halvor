//! High-level service abstractions to reduce code duplication
//!
//! This module provides common patterns and abstractions used across all services
//! to eliminate repetitive code and centralize common operations.

use crate::config::{EnvConfig, HostConfig};
use crate::services::host;
use crate::utils::exec::{CommandExecutor, Executor};
use anyhow::{Context, Result};

/// Service context that provides common operations for all services
///
/// This struct encapsulates the common pattern of:
/// 1. Creating an executor for a host
/// 2. Getting host configuration
/// 3. Determining if execution is local or remote
/// 4. Providing helper methods for common operations
pub struct ServiceContext {
    pub hostname: String,
    pub executor: Executor,
    pub host_config: HostConfig,
    pub target_host: String,
    pub is_local: bool,
}

impl ServiceContext {
    /// Create a new service context for a hostname
    ///
    /// This is the main entry point for service operations. It:
    /// - Creates an executor (local or remote)
    /// - Loads host configuration
    /// - Determines target host information
    pub fn new(hostname: &str, config: &EnvConfig) -> Result<Self> {
        let executor = Executor::new(hostname, config)?;
        let host_config = host::get_host_config_or_error(hostname)?;
        let target_host = executor.target_host(hostname, config)?;
        let is_local = executor.is_local();

        Ok(Self {
            hostname: hostname.to_string(),
            executor,
            host_config,
            target_host,
            is_local,
        })
    }

    /// Get the executor reference for command execution
    pub fn exec(&self) -> &Executor {
        &self.executor
    }

    /// Get backup path from host config with helpful error message
    pub fn backup_path(&self) -> Result<&str> {
        self.host_config.backup_path.as_deref().with_context(|| {
            format!(
                "Backup path not configured for {}\n\nAdd to .env:\n  HOST_{}_BACKUP_PATH=\"/path/to/backups/{}\"",
                self.hostname,
                self.hostname.to_uppercase(),
                self.hostname
            )
        })
    }

    /// Get IP address from host config
    pub fn ip(&self) -> Option<&str> {
        self.host_config.ip.as_deref()
    }

    /// Get hostname from host config
    pub fn hostname_field(&self) -> Option<&str> {
        self.host_config.hostname.as_deref()
    }

    /// Get Tailscale hostname from host config
    pub fn tailscale(&self) -> Option<&str> {
        self.host_config.tailscale.as_deref()
    }

    /// Print operation start message
    pub fn print_start(&self, operation: &str) {
        if self.is_local {
            println!("{} locally on {}...", operation, self.hostname);
        } else {
            println!(
                "{} on {} ({})...",
                operation, self.hostname, self.target_host
            );
        }
        println!();
    }

    /// Print operation complete message
    pub fn print_complete(&self, operation: &str) {
        println!();
        println!("✓ {} complete for {}", operation, self.hostname);
    }
}

/// Helper trait for common Docker operations
///
/// This trait provides a consistent interface for Docker operations
/// that work with any CommandExecutor
pub trait DockerOps: CommandExecutor + Sized {
    /// Ensure Docker is running
    fn ensure_docker(&self) -> Result<()> {
        crate::services::docker::ensure_docker_running(self)
    }

    /// Get docker compose command
    fn compose_cmd(&self) -> Result<String> {
        crate::services::docker::get_compose_command(self)
    }

    /// List all containers
    fn list_containers(&self) -> Result<Vec<String>> {
        crate::services::docker::list_containers(self)
    }

    /// Check if container is running
    fn is_container_running(&self, container: &str) -> Result<bool> {
        crate::services::docker::is_container_running(self, container)
    }

    /// Stop all containers
    fn stop_all_containers(&self) -> Result<Vec<String>> {
        crate::services::docker::stop_all_containers(self)
    }

    /// Start containers
    fn start_containers(&self, container_ids: &[String]) -> Result<()> {
        crate::services::docker::start_containers(self, container_ids)
    }

    /// List all volumes
    fn list_volumes(&self) -> Result<Vec<String>> {
        crate::services::docker::list_volumes(self)
    }

    /// Backup a volume
    fn backup_volume(&self, volume: &str, backup_dir: &str) -> Result<()> {
        crate::services::docker::backup_volume(self, volume, backup_dir)
    }

    /// Restore a volume
    fn restore_volume(&self, volume: &str, backup_dir: &str) -> Result<()> {
        crate::services::docker::restore_volume(self, volume, backup_dir)
    }

    /// Get bind mounts from a container
    fn get_bind_mounts(&self, container: &str) -> Result<Vec<String>> {
        crate::services::docker::get_bind_mounts(self, container)
    }
}

// Implement DockerOps for Executor
impl DockerOps for Executor {}

/// Helper trait for file operations
///
/// This trait provides a consistent interface for file operations
/// that work with any CommandExecutor
/// Note: mkdir_p is already provided by CommandExecutor, so we don't redefine it here
pub trait FileOps: CommandExecutor {
    /// Check if path is a directory
    fn is_dir(&self, path: &str) -> Result<bool> {
        self.is_directory(path)
    }

    /// Check if path exists
    fn path_exists(&self, path: &str) -> Result<bool> {
        self.file_exists(path)
    }

    /// Read file contents
    fn read(&self, path: &str) -> Result<String> {
        self.read_file(path)
    }

    /// Write file contents
    fn write(&self, path: &str, contents: &[u8]) -> Result<()> {
        self.write_file(path, contents)
    }
}

// Implement FileOps for Executor
impl FileOps for Executor {}

/// Helper for host configuration operations
pub struct HostConfigOps;

impl HostConfigOps {
    /// Get host config or error with helpful message
    pub fn get_or_error(hostname: &str) -> Result<HostConfig> {
        host::get_host_config_or_error(hostname)
    }

    /// Get host config (optional)
    pub fn get(hostname: &str) -> Result<Option<HostConfig>> {
        host::get_host_config(hostname)
    }

    /// Store host config
    pub fn store(hostname: &str, config: &HostConfig) -> Result<()> {
        host::store_host_config(hostname, config)
    }

    /// Delete host config
    pub fn delete(hostname: &str) -> Result<()> {
        host::delete_host_config(hostname)
    }

    /// List all hosts
    pub fn list() -> Result<Vec<String>> {
        host::list_hosts()
    }
}
