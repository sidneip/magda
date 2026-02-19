use std::sync::Arc;
use std::time::Instant;
use cdrs_tokio::cluster::session::{TcpSessionBuilder, SessionBuilder};
use cdrs_tokio::cluster::NodeTcpConfigBuilder;
use cdrs_tokio::load_balancing::RoundRobinLoadBalancingStrategy;
use cdrs_tokio::frame::message_response::ResponseBody;
use serde_json::Value;

use crate::error::{MagdaError, Result};
use crate::components::data_grid::{QueryResult, ColumnInfo};

/// A column in a Cassandra table schema
#[derive(Clone, Debug)]
pub struct SchemaColumn {
    pub name: String,
    pub data_type: String,
    pub kind: String,
    pub position: i32,
    pub clustering_order: String,
}

/// Schema information for a Cassandra table
#[derive(Clone, Debug)]
pub struct TableSchema {
    pub columns: Vec<SchemaColumn>,
}

/// Validate that a string is a safe CQL identifier (keyspace or table name).
/// Accepts unquoted identifiers: starts with letter/underscore, followed by alphanumeric/underscores.
pub fn validate_cql_identifier(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(MagdaError::validation("CQL identifier cannot be empty"));
    }
    let first = name.chars().next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return Err(MagdaError::validation(format!(
            "Invalid CQL identifier '{}': must start with a letter or underscore", name
        )));
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(MagdaError::validation(format!(
            "Invalid CQL identifier '{}': only letters, digits, and underscores allowed", name
        )));
    }
    Ok(())
}

/// Wrapper for Cassandra session
pub struct CassandraSession {
    inner: Arc<cdrs_tokio::cluster::session::Session<
        cdrs_tokio::transport::TransportTcp,
        cdrs_tokio::cluster::TcpConnectionManager,
        RoundRobinLoadBalancingStrategy<cdrs_tokio::transport::TransportTcp, cdrs_tokio::cluster::TcpConnectionManager>,
    >>,
}

impl CassandraSession {
    /// Execute a query
    pub async fn query(&self, query: &str) -> Result<cdrs_tokio::frame::Envelope> {
        self.inner
            .query(query)
            .await
            .map_err(|e| MagdaError::QueryError(format!("Query failed: {}", e)))
    }
}

/// Create a new Cassandra session
pub async fn create_session(host: &str, port: u16) -> Result<CassandraSession> {
    tracing::info!("Creating connection to {}:{}", host, port);
    
    let contact_point = format!("{}:{}", host, port);
    
    // Configure connection to Cassandra instance
    let config = NodeTcpConfigBuilder::new()
        .with_contact_point(contact_point.into())
        .build()
        .await
        .map_err(|e| MagdaError::ConnectionError(format!("Failed to build config: {}", e)))?;

    // Create session with round-robin load balancing
    let session = TcpSessionBuilder::new(
        RoundRobinLoadBalancingStrategy::new(), 
        config
    )
    .build()
    .await
    .map_err(|e| MagdaError::ConnectionError(format!("Failed to create session: {}", e)))?;

    tracing::info!("Connected to Cassandra at {}:{}", host, port);
    
    Ok(CassandraSession {
        inner: Arc::new(session),
    })
}

/// List all keyspaces from the real database
pub async fn list_keyspaces(session: &CassandraSession) -> Result<Vec<String>> {
    tracing::debug!("Listing keyspaces from system_schema");
    
    let result = session.query("SELECT keyspace_name FROM system_schema.keyspaces").await?;
    
    let mut keyspaces = Vec::new();
    
    // Process the envelope to extract keyspace names
    if let ResponseBody::Result(res_result_body) = result.response_body()? {
        match res_result_body {
            cdrs_tokio::frame::message_result::ResResultBody::Rows(rows_result) => {
                tracing::info!("Found {} keyspaces in result", rows_result.rows_count);
                
                // Extract keyspace names from each row
                for (i, row) in rows_result.rows_content.iter().enumerate() {
                    if let Some(keyspace_bytes) = row.get(0) {
                        // Extract bytes from CBytes using as_slice() method
                        if let Some(bytes) = keyspace_bytes.as_slice() {
                            match String::from_utf8(bytes.to_vec()) {
                                Ok(keyspace_name) => {
                                    tracing::debug!("Found keyspace {}: {}", i + 1, keyspace_name);
                                    keyspaces.push(keyspace_name);
                                }
                                Err(e) => {
                                    tracing::warn!("Failed to convert bytes to string for keyspace row {}: {:?}", i, e);
                                }
                            }
                        }
                    }
                }
                
                tracing::debug!("Extracted {} keyspace names", keyspaces.len());
            },
            _ => {
                tracing::debug!("Result is not Rows type, no keyspaces to extract");
            }
        }
    }
    
    tracing::info!("Found {} keyspaces", keyspaces.len());
    
    Ok(keyspaces)
}

/// List all tables in a keyspace from the real database
pub async fn list_tables(session: &CassandraSession, keyspace: &str) -> Result<Vec<String>> {
    validate_cql_identifier(keyspace)?;
    tracing::info!("Listing tables for keyspace: {}", keyspace);

    let query = format!("SELECT table_name FROM system_schema.tables WHERE keyspace_name = '{}'", keyspace);
    
    let result = session.query(&query).await?;
    
    let mut tables = Vec::new();
    
    // Process the envelope to extract table names
    if let ResponseBody::Result(res_result_body) = result.response_body()? {
        // Extract real table names from the result
        match res_result_body {
            cdrs_tokio::frame::message_result::ResResultBody::Rows(rows_result) => {
                tracing::info!("Found {} tables in result", rows_result.rows_count);
                
                // Extract table names from each row
                for (i, row) in rows_result.rows_content.iter().enumerate() {
                    if let Some(table_bytes) = row.get(0) {
                        // Extract bytes from CBytes using as_slice() method
                        if let Some(bytes) = table_bytes.as_slice() {
                            match String::from_utf8(bytes.to_vec()) {
                                Ok(table_name) => {
                                    tracing::debug!("Found table {}: {}", i + 1, table_name);
                                    tables.push(table_name);
                                }
                                Err(e) => {
                                    tracing::warn!("Failed to convert bytes to string for row {}: {:?}", i, e);
                                }
                            }
                        }
                    }
                }
                
                tracing::debug!("Extracted {} table names", tables.len());
            },
            _ => {
                tracing::debug!("Result is not Rows type, no tables to extract");
            }
        }
    }
    
    tracing::info!("Found {} tables in keyspace '{}'", tables.len(), keyspace);
    
    Ok(tables)
}

/// Describe a table's columns from system_schema.columns
pub async fn describe_table(session: &CassandraSession, keyspace: &str, table: &str) -> Result<TableSchema> {
    validate_cql_identifier(keyspace)?;
    validate_cql_identifier(table)?;
    tracing::debug!("Describing table {}.{}", keyspace, table);

    let query = format!(
        "SELECT column_name, type, kind, position, clustering_order FROM system_schema.columns WHERE keyspace_name = '{}' AND table_name = '{}'",
        keyspace, table
    );

    let result = session.query(&query).await?;
    let mut columns = Vec::new();

    if let ResponseBody::Result(res_result_body) = result.response_body()? {
        if let cdrs_tokio::frame::message_result::ResResultBody::Rows(rows_result) = res_result_body {
            for row in rows_result.rows_content.iter() {
                let get_string = |idx: usize| -> String {
                    row.get(idx)
                        .and_then(|b| b.as_slice())
                        .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok())
                        .unwrap_or_default()
                };
                let position = row.get(3)
                    .and_then(|b| b.as_slice())
                    .map(|bytes| {
                        if bytes.len() == 4 {
                            i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
                        } else {
                            0
                        }
                    })
                    .unwrap_or(0);

                columns.push(SchemaColumn {
                    name: get_string(0),
                    data_type: get_string(1),
                    kind: get_string(2),
                    position,
                    clustering_order: get_string(4),
                });
            }
        }
    }

    // Sort: partition_key by position, then clustering by position, then static, then regular
    columns.sort_by(|a, b| {
        let kind_order = |k: &str| match k {
            "partition_key" => 0,
            "clustering" => 1,
            "static" => 2,
            _ => 3,
        };
        kind_order(&a.kind).cmp(&kind_order(&b.kind))
            .then(a.position.cmp(&b.position))
            .then(a.name.cmp(&b.name))
    });

    tracing::info!("Described {}.{}: {} columns", keyspace, table, columns.len());
    Ok(TableSchema { columns })
}

/// Execute a CQL query and return results
pub async fn execute_query(session: &CassandraSession, query: &str) -> Result<QueryResult> {
    let start = Instant::now();
    tracing::debug!("Executing query: {}", query);
    
    let result = session.query(query).await?;
    
    let execution_time = start.elapsed().as_millis() as u64;
    
    let mut columns = Vec::new();
    let mut rows = Vec::new();
    let mut row_count = 0;

    // Process the envelope to extract data
    if let ResponseBody::Result(res_result_body) = result.response_body()? {
        match res_result_body {
            cdrs_tokio::frame::message_result::ResResultBody::Rows(rows_result) => {
                tracing::info!("Found {} rows in result", rows_result.rows_count);
                
                // Extract column information
                if !rows_result.metadata.col_specs.is_empty() {
                    let col_specs = &rows_result.metadata.col_specs;
                    for col_spec in col_specs {
                        columns.push(ColumnInfo {
                            name: col_spec.name.clone(),
                            data_type: format_col_type(&col_spec.col_type),
                        });
                    }
                }
                
                // Extract row data
                for row in rows_result.rows_content.iter() {
                    let mut row_data = Vec::new();
                    
                    for (i, column) in columns.iter().enumerate() {
                        if let Some(cell_bytes) = row.get(i) {
                            if let Some(bytes) = cell_bytes.as_slice() {
                                // Convert bytes based on column type
                                let value = convert_cassandra_value(bytes, &column.data_type);
                                row_data.push(value);
                            } else {
                                row_data.push(Value::Null);
                            }
                        } else {
                            row_data.push(Value::Null);
                        }
                    }
                    
                    rows.push(row_data);
                }
                
                row_count = rows.len();
                tracing::debug!("Extracted {} columns and {} rows", columns.len(), row_count);
            },
            _ => {
                tracing::debug!("Result is not Rows type, no data to extract");
            }
        }
    }
    
    // If no columns were found, create a default result
    if columns.is_empty() {
        columns.push(ColumnInfo {
            name: "result".to_string(),
            data_type: "text".to_string(),
        });
        rows.push(vec![Value::String("Query executed successfully".to_string())]);
        row_count = 1;
    }
    
    tracing::info!("Query executed in {}ms, {} rows returned", execution_time, row_count);
    
    Ok(QueryResult {
        columns,
        rows,
        execution_time_ms: execution_time,
        row_count,
    })
}

/// Test the connection by executing a simple system query
pub async fn test_connection(session: &CassandraSession) -> Result<()> {
    tracing::debug!("Testing connection with system query");
    
    let result = session.query("SELECT release_version FROM system.local").await?;
    
    // Try to extract version information
    if let ResponseBody::Result(res_result_body) = result.response_body()? {
        if let Some(rows_metadata) = res_result_body.as_rows_metadata() {
            tracing::debug!("Connection test: {} columns returned", rows_metadata.columns_count);
            // For now, just confirm connection works
            return Ok(());
        }
    }
    
    tracing::debug!("Connection test passed");
    Ok(())
}

/// Format a ColTypeOption into a clean, human-readable type name
fn format_col_type(col_type: &cdrs_tokio::frame::message_result::ColTypeOption) -> String {
    format!("{:?}", col_type.id).to_lowercase()
}

/// Convert Cassandra bytes to appropriate JSON value based on column type
fn convert_cassandra_value(bytes: &[u8], column_type: &str) -> Value {
    // Text types
    if column_type == "varchar" || column_type == "text" || column_type == "ascii" {
        if let Ok(s) = String::from_utf8(bytes.to_vec()) {
            return Value::String(s);
        }
    }

    match column_type {
        "int" => {
            if bytes.len() == 4 {
                return Value::Number(i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]).into());
            }
        }
        "bigint" | "counter" => {
            if bytes.len() == 8 {
                let val = i64::from_be_bytes(bytes[..8].try_into().unwrap());
                return Value::Number(val.into());
            }
        }
        "smallint" => {
            if bytes.len() == 2 {
                return Value::Number(i16::from_be_bytes([bytes[0], bytes[1]]).into());
            }
        }
        "tinyint" => {
            if bytes.len() == 1 {
                return Value::Number((bytes[0] as i8).into());
            }
        }
        "double" => {
            if bytes.len() == 8 {
                let val = f64::from_be_bytes(bytes[..8].try_into().unwrap());
                if let Some(num) = serde_json::Number::from_f64(val) {
                    return Value::Number(num);
                }
            }
        }
        "float" => {
            if bytes.len() == 4 {
                let val = f32::from_be_bytes(bytes[..4].try_into().unwrap());
                if let Some(num) = serde_json::Number::from_f64(val as f64) {
                    return Value::Number(num);
                }
            }
        }
        "decimal" => {
            // Cassandra decimal: first 4 bytes = scale (i32), rest = unscaled value (varint)
            if bytes.len() >= 4 {
                let scale = i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                let unscaled_bytes = &bytes[4..];
                // Convert varint to i64 (simplified, works for most practical values)
                let mut unscaled: i64 = 0;
                for &b in unscaled_bytes {
                    unscaled = (unscaled << 8) | (b as i64);
                }
                // Handle negative varint (if high bit is set)
                if !unscaled_bytes.is_empty() && unscaled_bytes[0] & 0x80 != 0 {
                    for _ in unscaled_bytes.len()..8 {
                        unscaled |= 0xFF << (unscaled_bytes.len() * 8 + (7 - unscaled_bytes.len()) * 8);
                    }
                    // Sign-extend
                    let shift = (8 - unscaled_bytes.len()) * 8;
                    unscaled = (unscaled << shift) >> shift;
                }
                let divisor = 10f64.powi(scale);
                let val = unscaled as f64 / divisor;
                return Value::String(format!("{:.prec$}", val, prec = scale.max(0) as usize));
            }
        }
        "boolean" => {
            if bytes.len() == 1 {
                return Value::Bool(bytes[0] != 0);
            }
        }
        "uuid" | "timeuuid" => {
            if bytes.len() == 16 {
                let mut uuid_bytes = [0u8; 16];
                uuid_bytes.copy_from_slice(bytes);
                return Value::String(uuid::Uuid::from_bytes(uuid_bytes).to_string());
            }
        }
        "timestamp" => {
            // Cassandra timestamp: milliseconds since epoch as i64
            if bytes.len() == 8 {
                let millis = i64::from_be_bytes(bytes[..8].try_into().unwrap());
                if let Some(dt) = chrono::DateTime::from_timestamp_millis(millis) {
                    return Value::String(dt.format("%Y-%m-%d %H:%M:%S").to_string());
                }
                return Value::Number(millis.into());
            }
        }
        "date" => {
            // Cassandra date: days since epoch (with offset of 2^31)
            if bytes.len() == 4 {
                let days = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                let epoch_days = days as i64 - (1 << 31);
                if let Some(date) = chrono::NaiveDate::from_num_days_from_ce_opt(epoch_days as i32 + 719_163) {
                    return Value::String(date.format("%Y-%m-%d").to_string());
                }
            }
        }
        "inet" => {
            if bytes.len() == 4 {
                return Value::String(format!("{}.{}.{}.{}", bytes[0], bytes[1], bytes[2], bytes[3]));
            } else if bytes.len() == 16 {
                let parts: Vec<String> = bytes.chunks(2)
                    .map(|c| format!("{:02x}{:02x}", c[0], c[1]))
                    .collect();
                return Value::String(parts.join(":"));
            }
        }
        "blob" => {
            return Value::String(format!("0x{}", hex::encode(bytes)));
        }
        "set" | "list" => {
            return Value::String(format!("[{} bytes]", bytes.len()));
        }
        "map" => {
            return Value::String(format!("{{{} bytes}}", bytes.len()));
        }
        _ => {}
    }

    // Fallback: try UTF-8, then hex
    if let Ok(s) = String::from_utf8(bytes.to_vec()) {
        Value::String(s)
    } else {
        Value::String(format!("0x{}", hex::encode(bytes)))
    }
}