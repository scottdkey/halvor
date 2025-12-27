// Halvor CLI Library
// CLI commands that orchestrate calls to halvor-core and halvor-agent
// No business logic here - all logic is in agent or core

mod cli_types;
mod commands;

pub use cli_types::Commands;

// Re-export commands module
pub use commands::handle_command;
