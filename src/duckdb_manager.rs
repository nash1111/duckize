use std::path::Path;
use duckdb::{Connection, Result};
use crate::data_preview::DataRange;

pub struct DuckDBManager {
    conn: Connection,
}

impl DuckDBManager {
    pub fn new() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Ok(Self { conn })
    }

    pub fn new_with_file(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        Ok(Self { conn })
    }

    pub fn import_csv(&self, 
        file_path: &Path, 
        table_name: &str, 
        range: &DataRange,
        has_header: bool
    ) -> Result<()> {
        let skip_rows = if has_header { range.start_row + 1 } else { range.start_row };
        let limit_rows = range.end_row - range.start_row + 1;
        
        let query = format!(
            "CREATE TABLE \"{}\" AS 
             SELECT * FROM read_csv('{}', 
                header = {}, 
                skip = {}
             )
             LIMIT {}",
            table_name,
            file_path.display(),
            has_header,
            skip_rows,
            limit_rows
        );

        self.conn.execute(&query, [])?;
        Ok(())
    }

    pub fn import_excel(&self,
        file_path: &Path,
        table_name: &str,
        sheet_name: &str,
        range: &DataRange,
        has_header: bool
    ) -> Result<()> {
        self.conn.execute("INSTALL spatial;", [])?;
        self.conn.execute("LOAD spatial;", [])?;
        
        let skip_rows = if has_header { range.start_row + 1 } else { range.start_row };
        let limit_rows = range.end_row - range.start_row + 1;
        
        let query = format!(
            "CREATE TABLE \"{}\" AS 
             SELECT * FROM st_read('{}', 
                layer = '{}',
                open_options = ['HEADERS={}']
             )
             LIMIT {} OFFSET {}",
            table_name,
            file_path.display(),
            sheet_name,
            if has_header { "FORCE" } else { "DISABLE" },
            limit_rows,
            skip_rows
        );

        self.conn.execute(&query, [])?;
        Ok(())
    }

    pub fn execute_query(&self, query: &str) -> Result<Vec<Vec<String>>> {
        // First check if the query is valid
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }
        
        // Prepare the statement with better error handling
        let stmt_result = self.conn.prepare(query);
        let mut stmt = match stmt_result {
            Ok(s) => s,
            Err(e) => {
                // Log the error and return empty result
                eprintln!("Failed to prepare query '{}': {}", query, e);
                return Ok(Vec::new());
            }
        };
        
        // Safely get column count using a more defensive approach
        let column_count = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| stmt.column_count())) {
            Ok(count) => count,
            Err(_) => {
                eprintln!("Failed to get column count for query: {}", query);
                return Ok(Vec::new());
            }
        };
        
        // Return empty result set if no columns
        if column_count == 0 {
            return Ok(Vec::new());
        }
        
        let rows = stmt.query_map([], |row| {
            let mut row_data = Vec::new();
            for i in 0..column_count {
                // Handle different value types and NULL values
                let value: String = match row.get_ref(i)? {
                    duckdb::types::ValueRef::Null => "NULL".to_string(),
                    duckdb::types::ValueRef::Boolean(b) => b.to_string(),
                    duckdb::types::ValueRef::TinyInt(n) => n.to_string(),
                    duckdb::types::ValueRef::SmallInt(n) => n.to_string(),
                    duckdb::types::ValueRef::Int(n) => n.to_string(),
                    duckdb::types::ValueRef::BigInt(n) => n.to_string(),
                    duckdb::types::ValueRef::HugeInt(n) => n.to_string(),
                    duckdb::types::ValueRef::UTinyInt(n) => n.to_string(),
                    duckdb::types::ValueRef::USmallInt(n) => n.to_string(),
                    duckdb::types::ValueRef::UInt(n) => n.to_string(),
                    duckdb::types::ValueRef::UBigInt(n) => n.to_string(),
                    duckdb::types::ValueRef::Float(f) => f.to_string(),
                    duckdb::types::ValueRef::Double(f) => f.to_string(),
                    duckdb::types::ValueRef::Decimal(d) => d.to_string(),
                    duckdb::types::ValueRef::Text(s) => String::from_utf8_lossy(s).to_string(),
                    duckdb::types::ValueRef::Blob(b) => format!("<BLOB {} bytes>", b.len()),
                    duckdb::types::ValueRef::Date32(d) => d.to_string(),
                    duckdb::types::ValueRef::Time64(_, t) => t.to_string(),
                    duckdb::types::ValueRef::Timestamp(_, t) => t.to_string(),
                    duckdb::types::ValueRef::Interval { months, days, nanos } => 
                        format!("{} months {} days {} ns", months, days, nanos),
                    _ => "<unsupported type>".to_string(),
                };
                row_data.push(value);
            }
            Ok(row_data)
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        
        Ok(results)
    }

    pub fn list_tables(&self) -> Result<Vec<String>> {
        // Use DuckDB's information_schema which is more reliable
        let query = "SELECT table_name FROM information_schema.tables WHERE table_schema = 'main' AND table_type = 'BASE TABLE'";
        
        match self.conn.prepare(query) {
            Ok(mut stmt) => {
                let tables = stmt.query_map([], |row| {
                    row.get::<_, String>(0)
                })?;

                let mut table_names = Vec::new();
                for table in tables {
                    if let Ok(name) = table {
                        table_names.push(name);
                    }
                }
                
                Ok(table_names)
            }
            Err(e) => {
                // Log error and return empty list
                eprintln!("Failed to list tables: {}", e);
                Ok(Vec::new())
            }
        }
    }

    pub fn get_table_schema(&self, table_name: &str) -> Result<Vec<(String, String)>> {
        // Use information_schema for more reliable schema retrieval
        let query = format!(
            "SELECT column_name, data_type 
             FROM information_schema.columns 
             WHERE table_schema = 'main' AND table_name = '{}' 
             ORDER BY ordinal_position",
            table_name.replace("'", "''")  // Escape single quotes
        );
        
        match self.conn.prepare(&query) {
            Ok(mut stmt) => {
                // Collect results first to avoid closure issues
                match stmt.query_map([], |row| {
                    let column_name: String = row.get(0)?;
                    let column_type: String = row.get(1)?;
                    Ok((column_name, column_type))
                }) {
                    Ok(mapped) => {
                        let result: Result<Vec<_>, _> = mapped.collect();
                        match result {
                            Ok(columns) => Ok(columns),
                            Err(e) => {
                                eprintln!("Query error getting schema for table '{}': {}", table_name, e);
                                Ok(Vec::new())
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to query schema for table '{}': {}", table_name, e);
                        Ok(Vec::new())
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to prepare schema query for table '{}': {}", table_name, e);
                Ok(Vec::new())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_tables_empty_database() {
        // Create a new in-memory database
        let db = DuckDBManager::new().expect("Failed to create DuckDB");
        
        // List tables - should return empty vector without crashing
        let tables = db.list_tables().expect("Failed to list tables");
        assert_eq!(tables.len(), 0);
    }

    #[test]
    fn test_list_tables_with_data() {
        // Create a new in-memory database
        let db = DuckDBManager::new().expect("Failed to create DuckDB");
        
        // Create a test table directly
        db.conn.execute("CREATE TABLE test_table (id INTEGER, name VARCHAR)", [])
            .expect("Failed to create table");
        
        // List tables - should return one table
        let tables = db.list_tables().expect("Failed to list tables");
        assert!(tables.contains(&"test_table".to_string()));
    }

    #[test]
    fn test_get_schema() {
        // Create a new in-memory database
        let db = DuckDBManager::new().expect("Failed to create DuckDB");
        
        // Create a test table
        db.conn.execute("CREATE TABLE test_table (id INTEGER, name VARCHAR)", [])
            .expect("Failed to create table");
        
        // Get schema
        let schema = db.get_table_schema("test_table").expect("Failed to get schema");
        assert_eq!(schema.len(), 2);
        
        // Check column names
        let col_names: Vec<String> = schema.iter().map(|(name, _)| name.clone()).collect();
        assert!(col_names.contains(&"id".to_string()));
        assert!(col_names.contains(&"name".to_string()));
    }
}