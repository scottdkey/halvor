use crate::core::table::Table;
use anyhow::Result;
use rusqlite::Connection;

/// Row builder for creating/updating rows with automatic field handling
/// This builder pattern allows you to only specify the fields you want to set,
/// and automatically handles id, created_at, updated_at, and merging with existing data
pub struct RowBuilder<T: Table + Clone> {
    row: Option<T>,
}

impl<T: Table + Clone> RowBuilder<T> {
    /// Create a new builder starting from existing row (if any)
    pub fn from_existing(existing: Option<&T>) -> Self {
        Self {
            row: existing.cloned(),
        }
    }

    /// Create a new builder with a default row
    /// The default row should have id, created_at, updated_at set to defaults
    /// which will be handled automatically by insert/update
    pub fn new(default: T) -> Self {
        Self { row: Some(default) }
    }

    /// Get the built row
    pub fn build(self) -> T {
        self.row.expect("RowBuilder must have a row")
    }
}

/// Helper macro to create a row builder with field updates
/// Usage: build_row!(HostInfoRow, existing, {
///     hostname: hostname.to_string(),
///     docker_version: docker_version.map(|s| s.to_string()),
/// })
#[macro_export]
macro_rules! build_row {
    ($row_type:ty, $existing:expr, { $($field:ident: $value:expr),* $(,)? }) => {{
        let mut row = if let Some(existing) = $existing {
            existing.clone()
        } else {
            // Create default row - id/created_at/updated_at will be set automatically
            <$row_type>::default_or_new()
        };

        $(
            row.$field = $value;
        )*

        row
    }};
}
