// Build service module - handles all build operations for different platforms
pub mod android;
pub mod app_store;
pub mod apple;
pub mod cli;
pub mod common;
pub mod github;
pub mod web;

// Re-export commonly used functions
pub use android::{build_android, sign_android};
pub use apple::{build_and_sign_ios, build_and_sign_mac, push_ios_to_app_store};
pub use cli::{build_cli, build_and_push_experimental};
pub use web::{build_web, build_web_docker, run_web_prod};
