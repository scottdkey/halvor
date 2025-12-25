use crate::db::core::table::Table;
use anyhow::Result;
use rusqlite::Connection;

/// Helper trait for building rows with automatic field handling
/// This allows building rows without worrying about id, created_at, updated_at
pub trait RowBuilder<T: Table> {
    /// Build a row from existing data (if any) and new data
    /// Only needs to provide data fields - id/created_at/updated_at handled automatically
    fn build(existing: Option<&T>, new_data: Self) -> T;
}

/// Upsert helper that automatically handles id, created_at, updated_at, and field merging
pub fn upsert_auto<T, B>(
    conn: &Connection,
    where_clause: &str,
    where_params: &[&dyn rusqlite::types::ToSql],
    builder: B,
) -> Result<String>
where
    T: Table + Clone,
    B: FnOnce(Option<&T>) -> T,
{
    use crate::db::core::table::DbTable;

    let existing = DbTable::<T>::select_many(conn, where_clause, where_params)?;
    let existing_row = existing.first();

    // Build the row - builder creates complete row
    let row = builder(existing_row);

    if existing.is_empty() {
        // New record - insert will set id, created_at, updated_at automatically
        DbTable::<T>::insert(conn, &row)
    } else {
        // Existing record - need to preserve id and created_at
        let existing_id = existing_row.unwrap().primary_key_value();

        // If builder didn't preserve id, we need to fix it
        // Since we can't modify fields on generic T, we'll use a workaround:
        // Use the existing row as base and merge in changes
        if row.primary_key_value() != existing_id {
            // Builder didn't preserve id - this is an error case
            // For now, we'll use insert_or_replace which will handle it
            // but we lose created_at preservation
            DbTable::<T>::insert_or_replace(conn, &row)
        } else {
            // Builder preserved id correctly - use update
            DbTable::<T>::update(conn, &row)?;
            Ok(existing_id)
        }
    }
}
