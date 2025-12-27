// Web platform module - build and dev operations

pub mod build;
pub mod dev;

// Re-export build functions
pub use build::{build_web, build_web_docker, run_web_prod};

// Re-export dev functions
pub use dev::{dev_web_bare_metal, dev_web_docker, dev_web_prod};

