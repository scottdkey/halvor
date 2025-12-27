/// Trait for partial updates - allows updating only specific fields
/// This is automatically implemented for generated structs
pub trait PartialUpdate<T: crate::core::table::Table> {
    /// Merge this partial update with an existing row, or create a new row with defaults
    fn merge(self, existing: Option<&T>) -> T;
}

/// Macro to create a partial update closure for upsert_by
/// Usage: upsert_fields!(RowType, existing, { field1: value1, field2: value2 })
/// Only the specified fields need to be provided - all others are preserved from existing or use defaults
#[macro_export]
macro_rules! upsert_fields {
    ($row_type:ty, $existing:ident, { $($field:ident: $value:expr),* $(,)? }) => {
        |$existing: Option<&$row_type>| {
            use crate::core::table::Table;
            use std::default::Default;

            // Start with existing if available, otherwise create new with defaults
            let mut row = $existing.cloned().unwrap_or_else(|| {
                // For new rows, we need to provide defaults
                // Since we can't use Default trait generically, we'll use a helper
                $crate::core::partial_update::create_default_row::<$row_type>()
            });

            // Update only the specified fields
            $(
                row.$field = $value;
            )*

            row
        }
    };
}

/// Helper function to create a default row for a table type
/// This is used when creating new rows in upsert_fields
pub fn create_default_row<T: crate::core::table::Table + Clone>() -> T {
    // This is a placeholder - we'll need to generate actual default constructors
    // For now, this will need to be implemented per-type or use a different approach
    unimplemented!("Default row creation must be implemented per table type")
}
