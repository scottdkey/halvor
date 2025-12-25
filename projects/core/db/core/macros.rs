/// Macro to automatically implement the Table trait from struct field names
///
/// Usage:
/// ```rust
/// #[derive(Debug, Clone)]
/// pub struct MyTable {
///     pub id: String,
///     pub key: String,
///     pub value: String,
///     pub created_at: i64,
///     pub updated_at: i64,
/// }
///
/// impl_table_auto!(MyTable, "my_table", [key, value]);
/// ```
///
/// This automatically generates all Table trait methods based on the field order:
/// - Fields are assumed to be in order: id, [custom fields], created_at, updated_at
/// - from_row() reads fields in order
/// - to_insert_params() and to_update_params() exclude id, created_at, updated_at
/// - All tables automatically get UUID primary key (id) and timestamp columns
#[macro_export]
macro_rules! impl_table_auto {
    (
        $struct_name:ident,
        $table_name:expr,
        [$($field:ident),*]
    ) => {
        impl $crate::db::core::table::Table for $struct_name {
            fn table_name() -> &'static str {
                $table_name
            }

            fn primary_key() -> &'static str {
                "id"
            }

            fn primary_key_value(&self) -> String {
                self.id.clone()
            }

            fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
                let mut idx = 0;
                Ok($struct_name {
                    id: { let v = row.get(idx)?; idx += 1; v },
                    $(
                        $field: { let v = row.get(idx)?; idx += 1; v },
                    )*
                    created_at: { let v = row.get(idx)?; idx += 1; v },
                    updated_at: { let v = row.get(idx)?; v },
                })
            }

            fn to_insert_params(&self) -> Vec<Box<dyn rusqlite::types::ToSql + Send + Sync>> {
                vec![
                    $(
                        Box::new(self.$field.clone()),
                    )*
                ]
            }

            fn to_update_params(&self) -> Vec<Box<dyn rusqlite::types::ToSql + Send + Sync>> {
                vec![
                    $(
                        Box::new(self.$field.clone()),
                    )*
                ]
            }

            fn insert_columns() -> &'static [&'static str] {
                &[
                    $(
                        stringify!($field),
                    )*
                ]
            }

            fn update_columns() -> &'static [&'static str] {
                &[
                    $(
                        stringify!($field),
                    )*
                ]
            }

            fn all_columns() -> &'static [&'static str] {
                &[
                    "id",
                    $(
                        stringify!($field),
                    )*
                    "created_at",
                    "updated_at",
                ]
            }

            fn column_indices() -> &'static [usize] {
                // Column indices are sequential: id=0, fields=1..N, created_at=N+1, updated_at=N+2
                // This is computed at compile time based on the number of fields
                const FIELD_COUNT: usize = [$(stringify!($field),)*].len();
                const TOTAL_COLS: usize = FIELD_COUNT + 3; // id + fields + created_at + updated_at
                const INDICES: [usize; TOTAL_COLS] = {
                    let mut arr = [0; TOTAL_COLS];
                    arr[0] = 0; // id
                    let mut i = 1;
                    while i <= FIELD_COUNT {
                        arr[i] = i;
                        i += 1;
                    }
                    arr[FIELD_COUNT + 1] = FIELD_COUNT + 1; // created_at
                    arr[FIELD_COUNT + 2] = FIELD_COUNT + 2; // updated_at
                    arr
                };
                &INDICES
            }
        }
    };
}
