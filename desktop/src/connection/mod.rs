use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{MagdaError, Result};

pub mod manager;

pub use manager::ConnectionManager;

/// Connection configuration for a Cassandra cluster
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub id: Uuid,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub keyspace: Option<String>,
    pub ssl_enabled: bool,
    pub connection_timeout_ms: u64,
    pub request_timeout_ms: u64,
    pub status_color: String,
    pub tag: String,
}

impl ConnectionConfig {
    /// Create a new connection configuration with default values
    pub fn new(name: impl Into<String>, host: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            host: host.into(),
            port: 9042,
            username: None,
            password: None,
            keyspace: None,
            ssl_enabled: false,
            connection_timeout_ms: 5000,
            request_timeout_ms: 12000,
            status_color: "#808080".to_string(),
            tag: String::new(),
        }
    }

    /// Set authentication credentials
    pub fn with_credentials(mut self, username: String, password: String) -> Self {
        self.username = Some(username);
        self.password = Some(password);
        self
    }

    /// Set the default keyspace
    pub fn with_keyspace(mut self, keyspace: String) -> Self {
        self.keyspace = Some(keyspace);
        self
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(MagdaError::validation("Connection name cannot be empty"));
        }

        if self.host.is_empty() {
            return Err(MagdaError::validation("Host cannot be empty"));
        }

        if self.port == 0 {
            return Err(MagdaError::validation("Port must be greater than 0"));
        }

        Ok(())
    }
}

/// Active Cassandra connection wrapper
pub struct CassandraConnection {
    pub id: Uuid,
    pub config: ConnectionConfig,
    connected_at: chrono::DateTime<chrono::Utc>,
    session: Option<crate::cassandra::CassandraSession>,
}

impl CassandraConnection {
    /// Create a new connection from configuration
    pub async fn connect(config: ConnectionConfig) -> Result<Self> {
        config.validate()?;

        // Create session using our cassandra module
        let session = crate::cassandra::create_session(&config.host, config.port).await?;

        // Set the active keyspace if configured
        if let Some(ref keyspace) = config.keyspace {
            crate::cassandra::validate_cql_identifier(keyspace)?;
            tracing::info!("Setting active keyspace to: {}", keyspace);
            session.query(&format!("USE {}", keyspace)).await?;
        }

        Ok(Self {
            id: config.id,
            config: config.clone(),
            connected_at: chrono::Utc::now(),
            session: Some(session),
        })
    }

    /// Test the connection by executing a simple query
    pub async fn test(&self) -> Result<()> {
        if let Some(ref session) = self.session {
            crate::cassandra::test_connection(session).await
        } else {
            Err(MagdaError::ConnectionError("No active session".to_string()))
        }
    }

    /// List all keyspaces
    pub async fn list_keyspaces(&self) -> Result<Vec<String>> {
        if let Some(ref session) = self.session {
            crate::cassandra::list_keyspaces(session).await
        } else {
            Err(MagdaError::ConnectionError("No active session".to_string()))
        }
    }

    /// List all tables in a keyspace
    pub async fn list_tables(&self, keyspace: &str) -> Result<Vec<String>> {
        if let Some(ref session) = self.session {
            crate::cassandra::list_tables(session, keyspace).await
        } else {
            Err(MagdaError::ConnectionError("No active session".to_string()))
        }
    }

    /// Resolve the keyspace to use: configured keyspace, or first non-system keyspace found.
    pub async fn resolve_keyspace(&self) -> Option<String> {
        if let Some(ref ks) = self.config.keyspace {
            return Some(ks.clone());
        }
        match self.list_keyspaces().await {
            Ok(keyspaces) => keyspaces
                .iter()
                .find(|ks| !ks.starts_with("system") && !ks.is_empty())
                .cloned(),
            Err(_) => None,
        }
    }

    /// Describe a table's schema (columns, types, keys)
    pub async fn describe_table(
        &self,
        keyspace: &str,
        table: &str,
    ) -> Result<crate::cassandra::TableSchema> {
        if let Some(ref session) = self.session {
            crate::cassandra::describe_table(session, keyspace, table).await
        } else {
            Err(MagdaError::ConnectionError("No active session".to_string()))
        }
    }

    /// Execute a CQL query and return results
    pub async fn execute_query(
        &self,
        query: &str,
    ) -> Result<crate::components::data_grid::QueryResult> {
        if let Some(ref session) = self.session {
            crate::cassandra::execute_query(session, query).await
        } else {
            Err(MagdaError::ConnectionError("No active session".to_string()))
        }
    }

    /// Get connection uptime
    pub fn uptime(&self) -> chrono::Duration {
        chrono::Utc::now() - self.connected_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_config_validation() {
        let valid_config = ConnectionConfig::new("Test", "localhost");
        assert!(valid_config.validate().is_ok());

        let invalid_config = ConnectionConfig::new("", "localhost");
        assert!(invalid_config.validate().is_err());

        let mut invalid_port = ConnectionConfig::new("Test", "localhost");
        invalid_port.port = 0;
        assert!(invalid_port.validate().is_err());
    }
}
