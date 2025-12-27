// Halvor Build and Dev Services
// Build operations and development mode for all platforms

// Platform modules (each contains build and dev submodules)
pub mod android;
pub mod apple;
pub mod cli;
pub mod web;

// Build utilities
pub mod docker;

// Shared utilities
pub mod common;
pub mod github;
pub mod zig;

// Re-export build functions
pub use android::{build_android, sign_android};
pub use apple::{build_and_sign_ios, build_and_sign_mac, push_ios_to_app_store};
pub use cli::{build_cli, build_and_push_experimental};
pub use web::{build_web, build_web_docker, run_web_prod};

// Re-export dev functions
pub use apple::{dev_ios, dev_mac};
pub use cli::dev_cli;
pub use web::{dev_web_bare_metal, dev_web_docker, dev_web_prod};

// Re-export docker build functions
pub use docker::build_container;

