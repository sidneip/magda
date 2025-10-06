use std::sync::Arc;
use std::time::Instant;
use cdrs_tokio::cluster::session::{TcpSessionBuilder, SessionBuilder};
use cdrs_tokio::cluster::NodeTcpConfigBuilder;
use cdrs_tokio::load_balancing::RoundRobinLoadBalancingStrategy;
use cdrs_tokio::frame::message_response::ResponseBody;
use serde_json::Value;

use crate::error::{MagdaError, Result};
use crate::components::data_grid::{QueryResult, ColumnInfo};

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
    tracing::info!("🔌 Creating real connection to {}:{}", host, port);
    
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

    tracing::info!("✅ Successfully connected to Cassandra at {}:{}", host, port);
    
    Ok(CassandraSession {
        inner: Arc::new(session),
    })
}

/// List all keyspaces from the real database
pub async fn list_keyspaces(session: &CassandraSession) -> Result<Vec<String>> {
    tracing::info!("📋 Listing keyspaces from system_schema");
    
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
                
                tracing::info!("✅ Successfully extracted {} keyspace names", keyspaces.len());
            },
            _ => {
                tracing::debug!("Result is not Rows type, no keyspaces to extract");
            }
        }
    }
    
    tracing::info!("✅ Found {} keyspaces", keyspaces.len());
    for keyspace in &keyspaces {
        tracing::debug!("  - Keyspace: {}", keyspace);
    }
    
    Ok(keyspaces)
}

/// List all tables in a keyspace from the real database
pub async fn list_tables(session: &CassandraSession, keyspace: &str) -> Result<Vec<String>> {
    tracing::info!("📋 Listing tables for keyspace: {} from system_schema", keyspace);
    
    // Use a simpler query without parameters for now
    let query = format!("SELECT table_name FROM system_schema.tables WHERE keyspace_name = '{}'", keyspace);
    
    let result = session.query(&query).await?;
    
    let mut tables = Vec::new();
    
    // Process the envelope to extract table names
    if let ResponseBody::Result(res_result_body) = result.response_body()? {
        tracing::debug!("ResResultBody: {:?}", res_result_body);
        
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
                
                tracing::info!("✅ Successfully extracted {} table names", tables.len());
            },
            _ => {
                tracing::debug!("Result is not Rows type, no tables to extract");
            }
        }
    }
    
    tracing::info!("✅ Found {} tables in keyspace '{}'", tables.len(), keyspace);
    for table in &tables {
        tracing::debug!("  - Table: {}", table);
    }
    
    Ok(tables)
}

/// Execute a CQL query and return results
pub async fn execute_query(session: &CassandraSession, query: &str) -> Result<QueryResult> {
    let start = Instant::now();
    tracing::info!("🔍 Executing real query: {}", query);
    
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
                            data_type: format!("{:?}", col_spec.col_type),
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
                tracing::info!("✅ Extracted {} columns and {} rows", columns.len(), row_count);
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
    
    tracing::info!("✅ Query executed successfully in {}ms, {} rows returned", execution_time, row_count);
    
    Ok(QueryResult {
        columns,
        rows,
        execution_time_ms: execution_time,
        row_count,
    })
}

/// Test the connection by executing a simple system query
pub async fn test_connection(session: &CassandraSession) -> Result<()> {
    tracing::info!("🧪 Testing connection with system query");
    
    let result = session.query("SELECT release_version FROM system.local").await?;
    
    // Try to extract version information
    if let ResponseBody::Result(res_result_body) = result.response_body()? {
        if let Some(rows_metadata) = res_result_body.as_rows_metadata() {
            tracing::info!("✅ Connection test query returned {} columns", rows_metadata.columns_count);
            // For now, just confirm connection works
            return Ok(());
        }
    }
    
    tracing::info!("✅ Connection test passed (no version info)");
    Ok(())
}

/// Convert Cassandra bytes to appropriate JSON value based on column type
fn convert_cassandra_value(bytes: &[u8], column_type: &str) -> Value {
    // First try UTF-8 string conversion for text types
    if column_type.contains("Text") || column_type.contains("Varchar") || column_type.contains("Ascii") {
        if let Ok(string_value) = String::from_utf8(bytes.to_vec()) {
            return Value::String(string_value);
        }
    }
    
    // Handle numeric types
    match column_type {
        t if t.contains("Int") => {
            if bytes.len() == 4 {
                let value = i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                return Value::Number(serde_json::Number::from(value));
            }
        }
        t if t.contains("Bigint") || t.contains("Counter") => {
            if bytes.len() == 8 {
                let value = i64::from_be_bytes([
                    bytes[0], bytes[1], bytes[2], bytes[3],
                    bytes[4], bytes[5], bytes[6], bytes[7]
                ]);
                return Value::Number(serde_json::Number::from(value));
            }
        }
        t if t.contains("Double") => {
            if bytes.len() == 8 {
                let value = f64::from_be_bytes([
                    bytes[0], bytes[1], bytes[2], bytes[3],
                    bytes[4], bytes[5], bytes[6], bytes[7]
                ]);
                if let Some(num) = serde_json::Number::from_f64(value) {
                    return Value::Number(num);
                }
            }
        }
        t if t.contains("Float") => {
            if bytes.len() == 4 {
                let value = f32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                if let Some(num) = serde_json::Number::from_f64(value as f64) {
                    return Value::Number(num);
                }
            }
        }
        t if t.contains("Boolean") => {
            if bytes.len() == 1 {
                return Value::Bool(bytes[0] != 0);
            }
        }
        t if t.contains("Uuid") => {
            if bytes.len() == 16 {
                let mut uuid_bytes = [0u8; 16];
                uuid_bytes.copy_from_slice(bytes);
                let uuid = uuid::Uuid::from_bytes(uuid_bytes);
                return Value::String(uuid.to_string());
            }
        }
        _ => {}
    }
    
    // Try UTF-8 string as fallback
    if let Ok(string_value) = String::from_utf8(bytes.to_vec()) {
        Value::String(string_value)
    } else {
        // Last resort: show as hex for truly binary data
        Value::String(format!("0x{}", hex::encode(bytes)))
    }
}