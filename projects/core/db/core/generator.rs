use crate::db;
use anyhow::Result;
use rusqlite::Connection;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug)]
struct ColumnInfo {
    name: String,
    sql_type: String,
    not_null: bool,
    _is_primary_key: bool,
    is_unique: bool,
}

#[derive(Debug)]
struct UniqueConstraint {
    columns: Vec<String>,
}

#[derive(Debug)]
struct TableInfo {
    _name: String,
    columns: Vec<ColumnInfo>,
    unique_constraints: Vec<UniqueConstraint>,
}

/// Get database schema using PRAGMA table_info (more reliable than parsing CREATE TABLE)
fn get_database_schema_from_pragma(conn: &Connection) -> Result<HashMap<String, TableInfo>> {
    let mut schemas = HashMap::new();

    // Get all table names
    let mut stmt = conn.prepare(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name != 'migrations'",
    )?;
    let tables: Vec<String> = stmt
        .query_map([], |row| Ok(row.get::<_, String>(0)?))?
        .collect::<Result<Vec<_>, _>>()?;

    for table_name in tables {
        let mut columns = Vec::new();
        let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", table_name))?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(1)?, // name
                row.get::<_, String>(2)?, // type
                row.get::<_, i32>(3)?,    // notnull
                row.get::<_, i32>(5)?,    // pk (primary key)
            ))
        })?;

        for row in rows {
            let (name, sql_type, not_null, pk) = row?;
            columns.push(ColumnInfo {
                name: name.clone(),
                sql_type: sql_type.to_uppercase(),
                not_null: not_null != 0,
                _is_primary_key: pk != 0,
                is_unique: false, // Will be set from indexes
            });
        }

        // Detect unique constraints from indexes
        let mut unique_constraints = Vec::new();
        let mut stmt = conn.prepare(&format!("PRAGMA index_list({})", table_name))?;
        let indexes = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(1)?, // name
                row.get::<_, i32>(2)?,    // unique
            ))
        })?;

        for index in indexes {
            let (index_name, is_unique) = index?;
            if is_unique != 0 {
                // Get columns for this index
                let mut stmt = conn.prepare(&format!("PRAGMA index_info({})", index_name))?;
                let index_cols = stmt.query_map([], |row| {
                    Ok(row.get::<_, String>(2)?) // column name
                })?;

                let mut constraint_cols = Vec::new();
                for col in index_cols {
                    constraint_cols.push(col?);
                }

                if !constraint_cols.is_empty() {
                    unique_constraints.push(UniqueConstraint {
                        columns: constraint_cols,
                    });
                }
            }
        }

        // Also check for UNIQUE in CREATE TABLE statement
        let create_sql: String = conn.query_row(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name = ?1",
            [&table_name],
            |row| row.get(0),
        )?;

        // Mark columns as unique if they have UNIQUE constraint in CREATE TABLE
        for col in &mut columns {
            if col.name != "id" {
                let col_upper = col.name.to_uppercase();
                let sql_upper = create_sql.to_uppercase();
                // Check for "column_name TEXT UNIQUE" or "UNIQUE(column_name)"
                if sql_upper.contains(&format!("{} UNIQUE", col_upper))
                    || sql_upper.contains(&format!("UNIQUE({})", col_upper))
                {
                    col.is_unique = true;
                }
            }
        }

        schemas.insert(
            table_name.clone(),
            TableInfo {
                _name: table_name,
                columns,
                unique_constraints,
            },
        );
    }

    Ok(schemas)
}

/// Generate Rust structs from database schema
pub fn generate_structs() -> Result<()> {
    let conn = db::get_connection()?;
    // Use PRAGMA table_info for more reliable schema detection
    let schemas = get_database_schema_from_pragma(&conn)?;

    let gen_dir = Path::new("src/db/generated");
    fs::create_dir_all(gen_dir)?;

    let mut mod_declarations = String::new();
    let mut exports = String::new();
    let mut table_names = Vec::new();

    for (table_name, table_info) in &schemas {
        table_names.push(table_name.clone());
        let struct_name = to_struct_name(&table_name);
        let file_name = format!("{}.rs", table_name);
        let file_path = gen_dir.join(&file_name);

        // Filter out id, created_at, updated_at from data fields
        let data_fields: Vec<&ColumnInfo> = table_info
            .columns
            .iter()
            .filter(|col| col.name != "id" && col.name != "created_at" && col.name != "updated_at")
            .collect();

        // Generate struct definition
        let mut struct_fields = String::new();
        let mut field_names = Vec::new();

        // Add id, created_at, updated_at first
        struct_fields.push_str("    pub id: String,\n");
        for field in &data_fields {
            let field_name = to_field_name(&field.name);
            let rust_type = sql_type_to_rust(&field.sql_type, field.not_null, &field.name);
            struct_fields.push_str(&format!("    pub {}: {},\n", field_name, rust_type));
            field_names.push(field_name);
        }
        struct_fields.push_str("    pub created_at: i64,\n");
        struct_fields.push_str("    pub updated_at: i64,\n");

        // Generate impl_table_auto call
        let field_list = field_names
            .iter()
            .map(|f| f.as_str())
            .collect::<Vec<_>>()
            .join(", ");

        // Check if this table needs Serialize/Deserialize (for encrypted_env_data)
        let serde_derive = if table_name == "encrypted_env_data" {
            "use serde::{Deserialize, Serialize};\n\n"
        } else {
            ""
        };
        let serde_attrs = if table_name == "encrypted_env_data" {
            "#[derive(Debug, Clone, Serialize, Deserialize)]"
        } else {
            "#[derive(Debug, Clone)]"
        };

        // Generate CRUD operations
        let operations = generate_crud_operations(
            &struct_name,
            table_name,
            &data_fields,
            &table_info.unique_constraints,
        );

        let struct_code = format!(
            r#"// Auto-generated from database schema
// This file is generated - do not edit manually
// Run `halvor db generate` to regenerate

use crate::impl_table_auto;
use crate::db;
use crate::db::core::table::DbTable;
use anyhow::Result;
{}

{}
pub struct {} {{
{}
}}

// Automatically implement Table trait from struct definition
impl_table_auto!(
    {},
    "{}",
    [{}]
);

{}
"#,
            serde_derive,
            serde_attrs,
            struct_name,
            struct_fields,
            struct_name,
            table_name,
            field_list,
            operations
        );

        fs::write(&file_path, struct_code).map_err(|e| {
            anyhow::anyhow!(
                "Failed to write generated file: {}: {}",
                file_path.display(),
                e
            )
        })?;

        mod_declarations.push_str(&format!("pub mod {};\n", table_name));
        exports.push_str(&format!("pub use {}::{};\n", table_name, struct_name));
        exports.push_str(&format!("pub use {}::{}Data;\n", table_name, struct_name));
        exports.push_str(&format!("pub use {}::{{insert_one, insert_many, upsert_one, select_one, select_many, delete_by_id}};\n", table_name));
    }

    // Generate mod.rs
    let mod_content = format!(
        r#"// Auto-generated module declarations
// This file is generated - do not edit manually
// Run `halvor db generate` to regenerate

{}

// Re-export all generated structs
{}
"#,
        mod_declarations, exports
    );

    fs::write(gen_dir.join("mod.rs"), mod_content)?;

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✓ Generated structs from database schema");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("Generated files in src/db/generated/:");
    for table_name in &table_names {
        println!("  • {}.rs", table_name);
    }
    println!();
    println!("You can now use these structs via:");
    println!("  use crate::db::generated::*;");

    Ok(())
}

/// Convert table name to struct name (snake_case to PascalCase + "Row" suffix)
fn to_struct_name(table_name: &str) -> String {
    let pascal = table_name
        .split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<String>();

    // Add "Row" suffix to match existing naming convention
    format!("{}Row", pascal)
}

/// Convert column name to field name (handle special cases)
fn to_field_name(column_name: &str) -> String {
    // Handle special cases like hostname_field
    if column_name == "hostname_field" {
        return "hostname_field".to_string();
    }
    column_name.to_string()
}

/// Convert SQL type to Rust type
fn sql_type_to_rust(sql_type: &str, not_null: bool, column_name: &str) -> String {
    // Special handling for boolean-like INTEGER fields
    let rust_type = if sql_type == "INTEGER" {
        match column_name {
            "tailscale_installed" | "portainer_installed" => "i32",
            _ => "i64",
        }
    } else {
        match sql_type {
            "TEXT" => "String",
            "REAL" => "f64",
            "BLOB" => "Vec<u8>",
            _ => "String", // Default to String for unknown types
        }
    };

    if not_null {
        rust_type.to_string()
    } else {
        format!("Option<{}>", rust_type)
    }
}

/// Generate all CRUD operations for a table
fn generate_crud_operations(
    struct_name: &str,
    _table_name: &str,
    data_fields: &[&ColumnInfo],
    unique_constraints: &[UniqueConstraint],
) -> String {
    let mut ops = String::new();

    // Generate data struct (only data fields, no id/created_at/updated_at)
    let mut data_struct_fields = String::new();
    let mut data_struct_params = String::new();
    for field in data_fields {
        let field_name = to_field_name(&field.name);
        let rust_type = sql_type_to_rust(&field.sql_type, field.not_null, &field.name);
        data_struct_fields.push_str(&format!("    pub {}: {},\n", field_name, rust_type));
        data_struct_params.push_str(&format!(
            "        {}: data.{}.clone(),\n",
            field_name, field_name
        ));
    }

    // Generate insert_one
    ops.push_str(&format!(
        r#"
/// Insert a new {} record
/// Only data fields are required - id, created_at, and updated_at are set automatically
pub fn insert_one(data: {}Data) -> Result<String> {{
    let conn = db::get_connection()?;
    let row = {} {{
        id: String::new(), // Set automatically
{}
        created_at: 0, // Set automatically
        updated_at: 0, // Set automatically
    }};
    DbTable::<{}>::insert(&conn, &row)
}}
"#,
        struct_name, struct_name, struct_name, data_struct_params, struct_name
    ));

    // Generate insert_many
    ops.push_str(&format!(
        r#"
/// Insert multiple {} records
pub fn insert_many(data_vec: Vec<{}Data>) -> Result<Vec<String>> {{
    let conn = db::get_connection()?;
    let mut ids = Vec::new();
    for data in data_vec {{
        let row = {} {{
            id: String::new(), // Set automatically
{}
            created_at: 0, // Set automatically
            updated_at: 0, // Set automatically
        }};
        ids.push(DbTable::<{}>::insert(&conn, &row)?);
    }}
    Ok(ids)
}}
"#,
        struct_name, struct_name, struct_name, data_struct_params, struct_name
    ));

    // Generate upsert_one
    let field_updates = generate_field_updates(data_fields);

    // Generate default struct initialization (empty/zero values)
    let mut default_fields = String::new();
    for field in data_fields {
        let field_name = to_field_name(&field.name);
        let rust_type = sql_type_to_rust(&field.sql_type, field.not_null, &field.name);
        if rust_type.starts_with("Option<") {
            default_fields.push_str(&format!("                {}: None,\n", field_name));
        } else if rust_type == "String" {
            default_fields.push_str(&format!("                {}: String::new(),\n", field_name));
        } else if rust_type == "i64" || rust_type == "i32" {
            default_fields.push_str(&format!("                {}: 0,\n", field_name));
        } else {
            default_fields.push_str(&format!(
                "                {}: Default::default(),\n",
                field_name
            ));
        }
    }

    // Generate initial assignments from data
    let mut initial_assignments = String::new();
    for field in data_fields {
        let field_name = to_field_name(&field.name);
        initial_assignments.push_str(&format!(
            "                r.{} = data.{}.clone();\n",
            field_name, field_name
        ));
    }

    ops.push_str(&format!(
        r#"
/// Upsert a {} record (insert if new, update if exists)
/// Only data fields are required - id, created_at, and updated_at are handled automatically
pub fn upsert_one(where_clause: &str, where_params: &[&dyn rusqlite::types::ToSql], data: {}Data) -> Result<String> {{
    let conn = db::get_connection()?;
    DbTable::<{}>::upsert_by(
        &conn,
        where_clause,
        where_params,
        |existing| {{
            let mut row = existing.cloned().unwrap_or_else(|| {{
                let mut r = {} {{
                    id: String::new(), // Set automatically
{}
                    created_at: 0, // Set automatically
                    updated_at: 0, // Set automatically
                }};
                // Set initial values from data
{}
                r
            }});
            // Update only the data fields
{}
            row
        }},
    )
}}
"#,
        struct_name, struct_name, struct_name, struct_name, default_fields, initial_assignments, field_updates
    ));

    // Generate select_one
    ops.push_str(&format!(
        r#"
/// Select one {} record
pub fn select_one(where_clause: &str, params: &[&dyn rusqlite::types::ToSql]) -> Result<Option<{}>> {{
    let conn = db::get_connection()?;
    DbTable::<{}>::select_one(&conn, where_clause, params)
}}
"#,
        struct_name, struct_name, struct_name
    ));

    // Generate select_many
    ops.push_str(&format!(
        r#"
/// Select many {} records
pub fn select_many(where_clause: &str, params: &[&dyn rusqlite::types::ToSql]) -> Result<Vec<{}>> {{
    let conn = db::get_connection()?;
    DbTable::<{}>::select_many(&conn, where_clause, params)
}}
"#,
        struct_name, struct_name, struct_name
    ));

    // Generate delete_by_id
    ops.push_str(&format!(
        r#"
/// Delete {} record by primary key (id)
pub fn delete_by_id(id: &str) -> Result<usize> {{
    let conn = db::get_connection()?;
    DbTable::<{}>::delete_many(&conn, "id = ?1", &[&id as &dyn rusqlite::types::ToSql])
}}
"#,
        struct_name, struct_name
    ));

    // Generate delete functions for unique constraints
    for constraint in unique_constraints {
        if constraint.columns.len() == 1 {
            // Single column unique constraint
            let col_name = &constraint.columns[0];
            let field_name = to_field_name(col_name);
            ops.push_str(&format!(
                r#"
/// Delete {} record by unique key: {}
pub fn delete_by_{}({}_value: &str) -> Result<usize> {{
    let conn = db::get_connection()?;
    DbTable::<{}>::delete_many(&conn, "{} = ?1", &[&{}_value as &dyn rusqlite::types::ToSql])
}}

"#,
                struct_name, col_name, field_name, field_name, struct_name, col_name, field_name
            ));
        }
    }

    // Generate data struct
    ops.insert_str(
        0,
        &format!(
            r#"
/// Data structure for {} operations (excludes id, created_at, updated_at)
#[derive(Debug, Clone)]
pub struct {}Data {{
{}
}}
"#,
            struct_name, struct_name, data_struct_fields
        ),
    );

    ops
}

/// Generate field update assignments for upsert
fn generate_field_updates(data_fields: &[&ColumnInfo]) -> String {
    let mut updates = String::new();
    for field in data_fields {
        let field_name = to_field_name(&field.name);
        updates.push_str(&format!(
            "            row.{} = data.{};\n",
            field_name, field_name
        ));
    }
    updates
}
