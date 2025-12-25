use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand, Clone)]
pub enum GenerateCommands {
    /// Generate FFI bindings for all platforms
    FfiBindings,
    /// Generate migration declarations
    Migrations,
    /// Generate everything (migrations + FFI bindings)
    All,
}

pub fn handle_generate(command: GenerateCommands) -> Result<()> {
    match command {
        GenerateCommands::FfiBindings => {
            println!("Generating FFI bindings...");
            crate::utils::ffi_bindings::generate_ffi_bindings_cli()?;
            println!("✓ FFI bindings generated");
        }
        GenerateCommands::Migrations => {
            println!("Generating migration declarations...");
            crate::utils::migrations::generate_migrations_cli()?;
            println!("✓ Migration declarations generated");
        }
        GenerateCommands::All => {
            println!("Generating all build artifacts...");
            crate::utils::migrations::generate_migrations_cli()?;
            crate::utils::ffi_bindings::generate_ffi_bindings_cli()?;
            println!("✓ All build artifacts generated");
        }
    }

    Ok(())
}
