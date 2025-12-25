use anyhow::Result;
use rusqlite::Connection;
use std::collections::HashMap;

/// Column definition from database schema
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnDef {
    pub name: String,
    pub sql_type: String,
    pub not_null: bool,
    pub default_value: Option<String>,
    pub primary_key: bool,
}

/// Table schema from database
#[derive(Debug, Clone)]
pub struct TableSchema {
    pub name: String,
    pub columns: Vec<ColumnDef>,
}

/// Get the current database schema
pub fn get_database_schema(conn: &Connection) -> Result<HashMap<String, TableSchema>> {
    let mut schemas = HashMap::new();

    // Query all tables
    let mut stmt = conn.prepare(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name != 'migrations'",
    )?;
    let tables: Vec<String> = stmt
        .query_map([], |row| Ok(row.get::<_, String>(0)?))?
        .collect::<Result<Vec<_>, _>>()?;

    for table_name in tables {
        let schema = get_table_schema(conn, &table_name)?;
        schemas.insert(table_name, schema);
    }

    Ok(schemas)
}

/// Get schema for a specific table
fn get_table_schema(conn: &Connection, table_name: &str) -> Result<TableSchema> {
    let mut columns = Vec::new();

    // Get CREATE TABLE statement
    let create_sql: String = conn.query_row(
        "SELECT sql FROM sqlite_master WHERE type='table' AND name = ?1",
        [table_name],
        |row| row.get(0),
    )?;

    // Parse the CREATE TABLE statement to extract column definitions
    // This is a simplified parser - SQLite's schema is complex, but we can extract basics
    let sql_upper = create_sql.to_uppercase();
    let start = sql_upper.find("CREATE TABLE").unwrap_or(0);
    let paren_start = create_sql[start..].find('(').unwrap_or(0) + start;
    let paren_end = create_sql.rfind(')').unwrap_or(create_sql.len());

    let column_defs = &create_sql[paren_start + 1..paren_end];

    // Split by comma, but be careful of nested parentheses
    let mut current = String::new();
    let mut depth = 0;
    let mut parts = Vec::new();

    for ch in column_defs.chars() {
        match ch {
            '(' => {
                depth += 1;
                current.push(ch);
            }
            ')' => {
                depth -= 1;
                current.push(ch);
            }
            ',' if depth == 0 => {
                parts.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }

    for part in parts {
        let part = part.trim();
        if part.is_empty()
            || part.to_uppercase().starts_with("PRIMARY KEY")
            || part.to_uppercase().starts_with("UNIQUE")
            || part.to_uppercase().starts_with("FOREIGN KEY")
            || part.to_uppercase().starts_with("CHECK")
        {
            continue;
        }

        let mut col_def = ColumnDef {
            name: String::new(),
            sql_type: String::new(),
            not_null: false,
            default_value: None,
            primary_key: false,
        };

        let words: Vec<&str> = part.split_whitespace().collect();
        if words.is_empty() {
            continue;
        }

        col_def.name = words[0].trim_matches('"').trim_matches('`').to_string();

        if words.len() > 1 {
            // Handle INTEGER PRIMARY KEY AUTOINCREMENT - treat as TEXT for our UUID-based system
            let sql_type = words[1].to_uppercase();
            if sql_type == "INTEGER" && part.to_uppercase().contains("PRIMARY KEY") {
                // This is likely INTEGER PRIMARY KEY AUTOINCREMENT - we use TEXT PRIMARY KEY
                col_def.sql_type = "TEXT".to_string();
            } else {
                col_def.sql_type = sql_type;
            }
        }

        for word in &words[2..] {
            let word_upper = word.to_uppercase();
            if word_upper == "NOT" || word_upper == "NULL" {
                col_def.not_null = true;
            } else if word_upper == "PRIMARY" || word_upper == "KEY" {
                col_def.primary_key = true;
            } else if word_upper == "DEFAULT" {
                // Next word is default value
                if let Some(default_idx) = words.iter().position(|w| w.to_uppercase() == "DEFAULT")
                {
                    if default_idx + 1 < words.len() {
                        col_def.default_value = Some(words[default_idx + 1].to_string());
                    }
                }
            }
        }

        columns.push(col_def);
    }

    Ok(TableSchema {
        name: table_name.to_string(),
        columns,
    })
}

/// Compare schemas and generate migration SQL
pub fn diff_schemas(
    expected: &TableSchema,
    actual: Option<&TableSchema>,
) -> Result<(Vec<String>, Vec<String>)> {
    let mut up_sql = Vec::new();
    let mut down_sql = Vec::new();

    match actual {
        None => {
            // Table doesn't exist - create it
            let columns_sql: Vec<String> = expected
                .columns
                .iter()
                .map(|col| {
                    let mut col_sql = format!("{} {}", col.name, col.sql_type);
                    if col.not_null {
                        col_sql.push_str(" NOT NULL");
                    }
                    if col.primary_key {
                        col_sql.push_str(" PRIMARY KEY");
                    }
                    if let Some(ref default) = col.default_value {
                        col_sql.push_str(&format!(" DEFAULT {}", default));
                    }
                    col_sql
                })
                .collect();

            up_sql.push(format!(
                "CREATE TABLE IF NOT EXISTS {} ({})",
                expected.name,
                columns_sql.join(", ")
            ));

            down_sql.push(format!("DROP TABLE IF EXISTS {}", expected.name));
        }
        Some(actual) => {
            // Table exists - check for column differences
            let expected_cols: HashMap<&str, &ColumnDef> = expected
                .columns
                .iter()
                .map(|c| (c.name.as_str(), c))
                .collect();
            let actual_cols: HashMap<&str, &ColumnDef> = actual
                .columns
                .iter()
                .map(|c| (c.name.as_str(), c))
                .collect();

            // Find new columns
            for (name, col) in &expected_cols {
                if !actual_cols.contains_key(name) {
                    let mut col_sql = format!(
                        "ALTER TABLE {} ADD COLUMN {} {}",
                        actual.name, col.name, col.sql_type
                    );
                    if col.not_null {
                        col_sql.push_str(" NOT NULL");
                    }
                    if let Some(ref default) = col.default_value {
                        col_sql.push_str(&format!(" DEFAULT {}", default));
                    }
                    up_sql.push(col_sql);

                    // Down migration: drop column (SQLite doesn't support DROP COLUMN easily)
                    // We'll need to recreate the table
                    down_sql.push(format!(
                        "-- Note: SQLite doesn't support DROP COLUMN. Manual intervention required for {}",
                        col.name
                    ));
                }
            }

            // Find removed columns (warn only, as SQLite doesn't support DROP COLUMN easily)
            for (name, _) in &actual_cols {
                if !expected_cols.contains_key(name) {
                    up_sql.push(format!(
                        "-- Warning: Column {} exists in database but not in struct. Manual removal required.",
                        name
                    ));
                }
            }
        }
    }

    Ok((up_sql, down_sql))
}

/// Compare two database schemas and generate migration SQL
/// This is useful for comparing database states (e.g., before/after a migration)
pub fn generate_migration_from_schema_diff(
    current_schema: &HashMap<String, TableSchema>,
    target_schema: &HashMap<String, TableSchema>,
) -> Result<Option<(Vec<String>, Vec<String>)>> {
    let mut all_up_sql = Vec::new();
    let mut all_down_sql = Vec::new();

    for (table_name, target) in target_schema {
        let current = current_schema.get(table_name);
        let (up_sql, down_sql) = diff_schemas(target, current)?;
        all_up_sql.extend(up_sql);
        all_down_sql.extend(down_sql);
    }

    if all_up_sql.is_empty() && all_down_sql.is_empty() {
        Ok(None)
    } else {
        Ok(Some((all_up_sql, all_down_sql)))
    }
}
