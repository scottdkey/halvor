use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand, Clone)]
pub enum GenerateCommands {
    /// Generate FFI bindings for all platforms
    FfiBindings,
    /// Generate migration declarations
    Migrations,
    /// Generate API client libraries (TypeScript, Kotlin, Swift)
    ApiClients,
    /// Generate everything (migrations + FFI bindings + API clients)
    All,
}

pub fn handle_generate(command: GenerateCommands) -> Result<()> {
    match command {
        GenerateCommands::FfiBindings => {
            println!("Generating FFI bindings...");
            halvor_core::utils::ffi_bindings::generate_ffi_bindings_cli()?;
            println!("✓ FFI bindings generated");
        }
        GenerateCommands::Migrations => {
            println!("Generating migration declarations...");
            halvor_db::migrations::generator::generate_migrations_cli()?;
            println!("✓ Migration declarations generated");
        }
        GenerateCommands::ApiClients => {
            println!("Generating API client libraries...");
            let workspace_root = std::env::current_dir()?;
            halvor_web::client_gen::generate_all_clients(&workspace_root)?;
            println!("✓ API client libraries generated");
            println!("  - TypeScript: projects/web/src/lib/halvor-api/client.ts");
            println!("  - Kotlin: projects/android/src/main/kotlin/dev/scottkey/halvor/api/HalvorApiClient.kt");
            println!("  - Swift: projects/ios/Sources/HalvorApi/HalvorApiClient.swift");
        }
        GenerateCommands::All => {
            println!("Generating all build artifacts...");
            halvor_db::migrations::generator::generate_migrations_cli()?;
            halvor_core::utils::ffi_bindings::generate_ffi_bindings_cli()?;
            let workspace_root = std::env::current_dir()?;
            halvor_web::client_gen::generate_all_clients(&workspace_root)?;
            println!("✓ All build artifacts generated");
        }
    }

    Ok(())
}
