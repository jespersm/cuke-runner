use gherkin::cuke;
use step::argument::FromStepArgumentError;

/// The lifetime parameter `'s` refers to the lifetime of the step.
/// It cannot escape the step function.
#[derive(Debug, Clone)]
pub struct DataTable<'s> {
    cuke_table: &'s cuke::Table<'s>,
}

impl<'s> From<&'s cuke::Table<'s>> for DataTable<'s> {
    fn from(cuke_table: &'s cuke::Table<'s>) -> Self {
        DataTable {
            cuke_table,
        }
    }
}

impl<'s> DataTable<'s> {
    pub fn to_vec<T: FromDataTableRow<'s>>(&'s self) -> super::FromStepArgumentResult<Vec<T>> {
        self.cuke_table.rows.iter()
            .skip(1)
            .map(|row| T::from_data_table_row(&row.cells))
            .collect()
    }

    pub fn headers<T: FromDataTableRow<'s>>(&self) -> super::FromStepArgumentResult<T> {
        if self.cuke_table.rows.len() >= 1 {
            T::from_data_table_row(&self.cuke_table.rows[0].cells)
        } else {
            Err(FromStepArgumentError{message: "No rows".to_string()})
        }
    }

    pub fn all_to_vec<T: FromDataTableRow<'s>>(&'s self) -> super::FromStepArgumentResult<Vec<T>> {
        self.cuke_table
            .rows
            .iter()
            .map(|row| T::from_data_table_row(&row.cells))
            .collect()
    }
}

/// Converts a row of the `DataTable` to `Self`.
///
/// The lifetime parameter `'r` refers to the lifetime of the DataTable row,
/// which is same as the DataTable itself. It cannot escape the step function.
pub trait FromDataTableRow<'r>: Sized {
    fn from_data_table_row<S: AsRef<str>>(row: &'r [S]) -> super::FromStepArgumentResult<Self>;
}
