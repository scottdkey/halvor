//! Database migration operations
//!
//! Core migration logic - these functions are called by command handlers.
//! Handles all migration-related operations including:
//! - Running all pending migrations
//! - Migrating up one step
//! - Migrating down one step
//! - Listing migrations with interactive selection

use crate::db;
use anyhow::{Context, Result};
use rusqlite::Connection;
use std::fs;
use std::path::PathBuf;

/// Run all pending migrations
pub fn migrate_all() -> Result<()> {
    let conn = db::get_connection()?;

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Running all pending migrations");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    db::migrations::run_migrations(&conn)?;

    println!();
    println!("✓ All migrations complete");

    Ok(())
}

/// Migrate up one step
pub fn migrate_up() -> Result<()> {
    let conn = db::get_connection()?;

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Migrating database up (one migration)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    db::migrations::migrate_up(&conn)?;

    println!();
    println!("✓ Migration complete");

    Ok(())
}

/// Migrate down one step
pub fn migrate_down() -> Result<()> {
    let conn = db::get_connection()?;

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Rolling back database (one migration)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    db::migrations::migrate_down(&conn)?;

    println!();
    println!("✓ Rollback complete");

    Ok(())
}

/// List migrations and allow interactive selection
pub fn migrate_list() -> Result<()> {
    let conn = db::get_connection()?;

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Migration Status");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    let status = db::migrations::get_migration_status(&conn)?;
    let current_version = db::migrations::get_current_migration_version(&conn)?;

    println!("Current version: {}", current_version);
    println!();
    println!(
        "{:<8} {:<40} {:<12} {:<12}",
        "Version", "Name", "Status", "Rollback"
    );
    println!("{}", "-".repeat(80));

    let mut migrations: Vec<(u32, String, bool, bool)> = Vec::new();
    for (version, name, is_applied, can_rollback) in status {
        let status_str = if is_applied {
            "✓ Applied"
        } else {
            "  Pending"
        };
        let rollback_str = if can_rollback { "Yes" } else { "No" };
        println!(
            "{:<8} {:<40} {:<12} {:<12}",
            version, name, status_str, rollback_str
        );

        migrations.push((version, name, is_applied, can_rollback));
    }

    println!();
    println!("Select a migration to roll forward or backward to:");
    println!("  Enter version number to migrate to that version");
    println!("  Or press Enter to exit");
    println!();

    use std::io::{self, Write};
    print!("Version: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();

    if input.is_empty() {
        println!("Exiting without changes.");
        return Ok(());
    }

    let target_version: u32 = input
        .parse()
        .with_context(|| format!("Invalid version number: {}", input))?;

    // Find the target migration
    let target_migration = migrations
        .iter()
        .find(|(v, _, _, _)| *v == target_version)
        .with_context(|| format!("Migration version {} not found", target_version))?;

    if target_migration.2 {
        // Migration is applied, roll back to before it
        println!();
        println!("Rolling back to version {}...", target_version);
        rollback_to_version(&conn, target_version)?;
    } else {
        // Migration is pending, migrate forward to it
        println!();
        println!("Migrating forward to version {}...", target_version);
        migrate_to_version(&conn, target_version)?;
    }

    Ok(())
}

/// Rollback to a specific version (exclusive - rolls back to before that version)
fn rollback_to_version(conn: &Connection, target_version: u32) -> Result<()> {
    let mut current_version = db::migrations::get_current_migration_version(conn)?;

    if current_version < target_version {
        anyhow::bail!(
            "Cannot rollback to version {} - current version is {}",
            target_version,
            current_version
        );
    }

    while current_version >= target_version {
        if current_version == 0 {
            break; // Already at the beginning
        }
        if current_version < target_version {
            break; // Reached target
        }
        db::migrations::migrate_down(conn)?;
        current_version = db::migrations::get_current_migration_version(conn)?;
    }

    println!("✓ Rolled back to version {}", target_version);
    Ok(())
}

/// Migrate forward to a specific version (inclusive)
fn migrate_to_version(conn: &Connection, target_version: u32) -> Result<()> {
    let mut current_version = db::migrations::get_current_migration_version(conn)?;

    if current_version >= target_version {
        anyhow::bail!(
            "Already at or past version {} - current version is {}",
            target_version,
            current_version
        );
    }

    while current_version < target_version {
        db::migrations::migrate_up(conn)?;
        current_version = db::migrations::get_current_migration_version(conn)?;
        if current_version >= target_version {
            break; // Reached target
        }
    }

    println!("✓ Migrated to version {}", target_version);
    Ok(())
}

/// Generate a new migration file
pub fn generate_migration(description: Vec<String>) -> Result<()> {
    if description.is_empty() {
        anyhow::bail!(
            "Migration description is required. Example: halvor db migrate generate add users table"
        );
    }

    let desc = description.join("_").to_lowercase().replace(" ", "_");
    create_migration_file(&desc, &[], &[])
}

/// Helper to create migration file
fn create_migration_file(desc: &str, up_sql: &[String], down_sql: &[String]) -> Result<()> {
    // Find the highest migration number
    let migrations_dir = PathBuf::from("src/db/migrations");
    let mut max_version = 0u32;

    if migrations_dir.exists() {
        for entry in fs::read_dir(&migrations_dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name.ends_with(".rs") && file_name != "mod.rs" {
                    let parts: Vec<&str> = file_name.trim_end_matches(".rs").split('_').collect();
                    if let Some(version_str) = parts.first() {
                        if let Ok(version) = version_str.parse::<u32>() {
                            max_version = max_version.max(version);
                        }
                    }
                }
            }
        }
    }

    let next_version = max_version + 1;
    let version_str = format!("{:03}", next_version);
    let file_name = format!("{}_{}.rs", version_str, desc);
    let file_path = migrations_dir.join(&file_name);

    // Create migration file content
    let up_content = if up_sql.is_empty() {
        r#"    // TODO: Implement migration
    // Example:
    // conn.execute(
    //     "CREATE TABLE IF NOT EXISTS example (
    //         id TEXT PRIMARY KEY,
    //         name TEXT NOT NULL,
    //         created_at INTEGER NOT NULL,
    //         updated_at INTEGER NOT NULL
    //     )",
    //     [],
    // )
    // .context("Failed to create example table")?;
    
    Ok(())"#
            .to_string()
    } else {
        let mut content = String::new();
        for sql in up_sql {
            if sql.starts_with("--") {
                content.push_str(&format!("    {}\n", sql));
            } else {
                content.push_str(&format!(
                    "    conn.execute(\n        {:?},\n        [],\n    )\n    .context(\"Failed to execute migration\")?;\n\n",
                    sql
                ));
            }
        }
        content.push_str("    Ok(())");
        content
    };

    let down_content = if down_sql.is_empty() {
        r#"    // TODO: Implement rollback
    // Example:
    // conn.execute("DROP TABLE IF EXISTS example", [])
    //     .context("Failed to drop example table")?;
    
    Ok(())"#
            .to_string()
    } else {
        let mut content = String::new();
        for sql in down_sql {
            if sql.starts_with("--") {
                content.push_str(&format!("    {}\n", sql));
            } else {
                content.push_str(&format!(
                    "    conn.execute(\n        {:?},\n        [],\n    )\n    .context(\"Failed to execute rollback\")?;\n\n",
                    sql
                ));
            }
        }
        content.push_str("    Ok(())");
        content
    };

    let content = format!(
        r#"use anyhow::{{Context, Result}};
use rusqlite::Connection;

/// Migration {:03}: {}
pub fn up(conn: &Connection) -> Result<()> {{
{}
}}

/// Rollback: {}
pub fn down(conn: &Connection) -> Result<()> {{
{}
}}
"#,
        next_version,
        desc.replace("_", " "),
        up_content,
        format!("Undo {}", desc.replace("_", " ")),
        down_content
    );

    // Write file
    fs::write(&file_path, content)
        .with_context(|| format!("Failed to write migration file: {}", file_path.display()))?;

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✓ Created migration file: {}", file_path.display());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("The migration will be automatically discovered on the next build.");

    Ok(())
}
