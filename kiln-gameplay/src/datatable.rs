//! Data tables with CSV import for items, enemies, loot, etc.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Data type for table columns
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DataType {
    Integer,
    Float,
    String,
    Boolean,
}

/// A data table column definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataTableColumn {
    pub name: String,
    pub data_type: DataType,
}

/// A single row in a data table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataTableRow {
    pub id: String,
    pub values: HashMap<String, DataValue>,
}

/// A value in a data table
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "value")]
pub enum DataValue {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
}

impl DataValue {
    /// Get the data type of this value
    pub fn data_type(&self) -> DataType {
        match self {
            DataValue::Integer(_) => DataType::Integer,
            DataValue::Float(_) => DataType::Float,
            DataValue::String(_) => DataType::String,
            DataValue::Boolean(_) => DataType::Boolean,
        }
    }

    /// Get as integer
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            DataValue::Integer(v) => Some(*v),
            _ => None,
        }
    }

    /// Get as float
    pub fn as_float(&self) -> Option<f64> {
        match self {
            DataValue::Float(v) => Some(*v),
            DataValue::Integer(v) => Some(*v as f64),
            _ => None,
        }
    }

    /// Get as string
    pub fn as_string(&self) -> Option<&str> {
        match self {
            DataValue::String(v) => Some(v),
            _ => None,
        }
    }

    /// Get as boolean
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            DataValue::Boolean(v) => Some(*v),
            _ => None,
        }
    }
}

/// A data table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataTable {
    pub id: String,
    pub columns: Vec<DataTableColumn>,
    pub rows: Vec<DataTableRow>,
}

#[derive(Error, Debug)]
pub enum DataTableError {
    #[error("Column not found: {0}")]
    ColumnNotFound(String),
    #[error("Row not found: {0}")]
    RowNotFound(String),
    #[error("Type mismatch for column {0}")]
    TypeMismatch(String),
    #[error("CSV parse error: {0}")]
    CsvError(#[from] csv::Error),
    #[error("Value parse error: {0}")]
    ValueParseError(String),
}

impl DataTable {
    /// Create a new data table
    pub fn new(id: String) -> Self {
        Self {
            id,
            columns: vec![],
            rows: vec![],
        }
    }

    /// Add a column to the table
    pub fn add_column(&mut self, column: DataTableColumn) {
        self.columns.push(column);
    }

    /// Add a row to the table
    pub fn add_row(&mut self, row: DataTableRow) {
        self.rows.push(row);
    }

    /// Get a row by ID
    pub fn get_row(&self, row_id: &str) -> Option<&DataTableRow> {
        self.rows.iter().find(|r| r.id == row_id)
    }

    /// Get a column by name
    pub fn get_column(&self, column_name: &str) -> Option<&DataTableColumn> {
        self.columns.iter().find(|c| c.name == column_name)
    }

    /// Get a value from a specific row and column
    pub fn get_value(&self, row_id: &str, column_name: &str) -> Option<&DataValue> {
        self.get_row(row_id)
            .and_then(|row| row.values.get(column_name))
    }

    /// Import from CSV string
    /// First row is column names, second row is data types (Integer, Float, String, Boolean)
    /// Subsequent rows are data
    pub fn import_from_csv(&mut self, csv_data: &str) -> Result<(), DataTableError> {
        let mut reader = csv::Reader::from_reader(csv_data.as_bytes());
        let headers = reader.headers()?.clone();

        // Parse column names and types
        self.columns.clear();
        self.rows.clear();

        for header in &headers {
            self.columns.push(DataTableColumn {
                name: header.to_string(),
                data_type: DataType::String, // Default, will be updated from second row
            });
        }

        // Read second row for data types
        let type_record = reader.records().next().ok_or(DataTableError::ValueParseError(
            "Missing data type row".to_string(),
        ))??;

        for (i, type_str) in type_record.iter().enumerate() {
            if i < self.columns.len() {
                self.columns[i].data_type = match type_str.to_lowercase().as_str() {
                    "integer" | "int" => DataType::Integer,
                    "float" | "double" => DataType::Float,
                    "string" | "str" => DataType::String,
                    "boolean" | "bool" => DataType::Boolean,
                    _ => DataType::String,
                };
            }
        }

        // Read data rows
        for result in reader.records() {
            let record = result?;
            let mut row = DataTableRow {
                id: record.get(0).unwrap_or("").to_string(),
                values: HashMap::new(),
            };

            for (i, value_str) in record.iter().enumerate() {
                if i < self.columns.len() {
                    let column = &self.columns[i];
                    let value = match column.data_type {
                        DataType::Integer => DataValue::Integer(value_str.parse().unwrap_or(0)),
                        DataType::Float => DataValue::Float(value_str.parse().unwrap_or(0.0)),
                        DataType::String => DataValue::String(value_str.to_string()),
                        DataType::Boolean => {
                            DataValue::Boolean(value_str.to_lowercase() == "true")
                        }
                    };
                    row.values.insert(column.name.clone(), value);
                }
            }

            self.rows.push(row);
        }

        Ok(())
    }

    /// Export to CSV string
    pub fn export_to_csv(&self) -> String {
        let mut writer = csv::Writer::from_writer(vec![]);

        // Write headers
        let headers: Vec<&str> = self.columns.iter().map(|c| c.name.as_str()).collect();
        writer.write_record(&headers).unwrap();

        // Write data types
        let types: Vec<&str> = self
            .columns
            .iter()
            .map(|c| match c.data_type {
                DataType::Integer => "Integer",
                DataType::Float => "Float",
                DataType::String => "String",
                DataType::Boolean => "Boolean",
            })
            .collect();
        writer.write_record(&types).unwrap();

        // Write rows
        for row in &self.rows {
            let record: Vec<String> = self
                .columns
                .iter()
                .map(|col| {
                    match row.values.get(&col.name) {
                        Some(DataValue::Integer(v)) => v.to_string(),
                        Some(DataValue::Float(v)) => v.to_string(),
                        Some(DataValue::String(v)) => v.clone(),
                        Some(DataValue::Boolean(v)) => v.to_string(),
                        None => String::new(),
                    }
                })
                .collect();
            writer.write_record(&record).unwrap();
        }

        String::from_utf8(writer.into_inner().unwrap()).unwrap()
    }

    /// Find rows by column value
    pub fn find_rows(&self, column_name: &str, value: &DataValue) -> Vec<&DataTableRow> {
        self.rows
            .iter()
            .filter(|row| row.values.get(column_name) == Some(value))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_table_creation() {
        let table = DataTable::new("items".to_string());
        assert_eq!(table.id, "items");
        assert_eq!(table.columns.len(), 0);
        assert_eq!(table.rows.len(), 0);
    }

    #[test]
    fn test_add_column() {
        let mut table = DataTable::new("items".to_string());
        table.add_column(DataTableColumn {
            name: "name".to_string(),
            data_type: DataType::String,
        });
        assert_eq!(table.columns.len(), 1);
        assert_eq!(table.columns[0].name, "name");
    }

    #[test]
    fn test_add_row() {
        let mut table = DataTable::new("items".to_string());
        table.add_column(DataTableColumn {
            name: "name".to_string(),
            data_type: DataType::String,
        });

        let mut row = DataTableRow {
            id: "item1".to_string(),
            values: HashMap::new(),
        };
        row.values.insert(
            "name".to_string(),
            DataValue::String("Sword".to_string()),
        );
        table.add_row(row);

        assert_eq!(table.rows.len(), 1);
    }

    #[test]
    fn test_get_row() {
        let mut table = DataTable::new("items".to_string());
        table.add_column(DataTableColumn {
            name: "name".to_string(),
            data_type: DataType::String,
        });

        let mut row = DataTableRow {
            id: "item1".to_string(),
            values: HashMap::new(),
        };
        row.values.insert(
            "name".to_string(),
            DataValue::String("Sword".to_string()),
        );
        table.add_row(row);

        let retrieved = table.get_row("item1");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "item1");
    }

    #[test]
    fn test_get_value() {
        let mut table = DataTable::new("items".to_string());
        table.add_column(DataTableColumn {
            name: "name".to_string(),
            data_type: DataType::String,
        });
        table.add_column(DataTableColumn {
            name: "damage".to_string(),
            data_type: DataType::Integer,
        });

        let mut row = DataTableRow {
            id: "item1".to_string(),
            values: HashMap::new(),
        };
        row.values.insert(
            "name".to_string(),
            DataValue::String("Sword".to_string()),
        );
        row.values.insert("damage".to_string(), DataValue::Integer(10));
        table.add_row(row);

        let name = table.get_value("item1", "name");
        assert_eq!(name, Some(&DataValue::String("Sword".to_string())));

        let damage = table.get_value("item1", "damage");
        assert_eq!(damage, Some(&DataValue::Integer(10)));
    }

    #[test]
    fn test_csv_import() {
        let csv_data = "id,name,damage,type\nString,String,Integer,String\nitem1,Sword,10,Weapon\nitem2,Shield,5,Armor";

        let mut table = DataTable::new("items".to_string());
        table.import_from_csv(csv_data).unwrap();

        assert_eq!(table.columns.len(), 4);
        assert_eq!(table.rows.len(), 2);

        let row1 = table.get_row("item1").unwrap();
        assert_eq!(
            row1.values.get("name"),
            Some(&DataValue::String("Sword".to_string()))
        );
        assert_eq!(row1.values.get("damage"), Some(&DataValue::Integer(10)));
    }

    #[test]
    fn test_csv_export() {
        let mut table = DataTable::new("items".to_string());
        table.add_column(DataTableColumn {
            name: "id".to_string(),
            data_type: DataType::String,
        });
        table.add_column(DataTableColumn {
            name: "name".to_string(),
            data_type: DataType::String,
        });
        table.add_column(DataTableColumn {
            name: "damage".to_string(),
            data_type: DataType::Integer,
        });

        let mut row = DataTableRow {
            id: "item1".to_string(),
            values: HashMap::new(),
        };
        row.values.insert("id".to_string(), DataValue::String("item1".to_string()));
        row.values.insert(
            "name".to_string(),
            DataValue::String("Sword".to_string()),
        );
        row.values.insert("damage".to_string(), DataValue::Integer(10));
        table.add_row(row);

        let csv = table.export_to_csv();
        assert!(csv.contains("id,name,damage"));
        assert!(csv.contains("String,String,Integer"));
        assert!(csv.contains("item1,Sword,10"));
    }

    #[test]
    fn test_find_rows() {
        let mut table = DataTable::new("items".to_string());
        table.add_column(DataTableColumn {
            name: "id".to_string(),
            data_type: DataType::String,
        });
        table.add_column(DataTableColumn {
            name: "type".to_string(),
            data_type: DataType::String,
        });

        let mut row1 = DataTableRow {
            id: "item1".to_string(),
            values: HashMap::new(),
        };
        row1.values.insert("id".to_string(), DataValue::String("item1".to_string()));
        row1.values.insert("type".to_string(), DataValue::String("Weapon".to_string()));

        let mut row2 = DataTableRow {
            id: "item2".to_string(),
            values: HashMap::new(),
        };
        row2.values.insert("id".to_string(), DataValue::String("item2".to_string()));
        row2.values.insert("type".to_string(), DataValue::String("Weapon".to_string()));

        let mut row3 = DataTableRow {
            id: "item3".to_string(),
            values: HashMap::new(),
        };
        row3.values.insert("id".to_string(), DataValue::String("item3".to_string()));
        row3.values.insert("type".to_string(), DataValue::String("Armor".to_string()));

        table.add_row(row1);
        table.add_row(row2);
        table.add_row(row3);

        let weapons = table.find_rows("type", &DataValue::String("Weapon".to_string()));
        assert_eq!(weapons.len(), 2);
    }

    #[test]
    fn test_data_value_conversions() {
        let int_val = DataValue::Integer(42);
        assert_eq!(int_val.as_integer(), Some(42));
        assert_eq!(int_val.as_float(), Some(42.0));
        assert_eq!(int_val.as_string(), None);
        assert_eq!(int_val.as_boolean(), None);

        let float_val = DataValue::Float(3.14);
        assert_eq!(float_val.as_integer(), None);
        assert_eq!(float_val.as_float(), Some(3.14));
        assert_eq!(float_val.as_string(), None);
        assert_eq!(float_val.as_boolean(), None);

        let string_val = DataValue::String("hello".to_string());
        assert_eq!(string_val.as_integer(), None);
        assert_eq!(string_val.as_float(), None);
        assert_eq!(string_val.as_string(), Some("hello"));
        assert_eq!(string_val.as_boolean(), None);

        let bool_val = DataValue::Boolean(true);
        assert_eq!(bool_val.as_integer(), None);
        assert_eq!(bool_val.as_float(), None);
        assert_eq!(bool_val.as_string(), None);
        assert_eq!(bool_val.as_boolean(), Some(true));
    }
}
