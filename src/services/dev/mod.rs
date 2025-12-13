// Development service module - handles development mode operations
pub mod apple;
pub mod cli;
pub mod web;

// Re-export commonly used functions
pub use apple::{dev_ios, dev_mac};
pub use cli::dev_cli;
pub use web::{dev_web_bare_metal, dev_web_docker, dev_web_prod};
