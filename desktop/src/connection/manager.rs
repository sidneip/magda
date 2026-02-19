use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::{CassandraConnection, ConnectionConfig};
use crate::error::{MagdaError, Result};

/// Wrapper for TOML serialization (TOML requires a root table)
#[derive(serde::Serialize, serde::Deserialize)]
struct SavedConnections {
    connections: Vec<ConnectionConfig>,
}

/// Get the path to the connections config file
fn connections_file_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("com", "magda", "Magda")
        .map(|dirs| dirs.config_dir().join("connections.toml"))
}

/// Save connection configs to disk
fn persist_configs(configs: &[ConnectionConfig]) {
    let Some(path) = connections_file_path() else {
        return;
    };

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let saved = SavedConnections {
        connections: configs.to_vec(),
    };
    match toml::to_string_pretty(&saved) {
        Ok(content) => {
            if let Err(e) = std::fs::write(&path, content) {
                warn!("Failed to save connections to disk: {}", e);
            } else {
                debug!("Saved {} connections to {:?}", configs.len(), path);
            }
        }
        Err(e) => warn!("Failed to serialize connections: {}", e),
    }
}

/// Load connection configs from disk
fn load_persisted_configs() -> Vec<ConnectionConfig> {
    let Some(path) = connections_file_path() else {
        return Vec::new();
    };

    match std::fs::read_to_string(&path) {
        Ok(content) => match toml::from_str::<SavedConnections>(&content) {
            Ok(saved) => {
                info!(
                    "Loaded {} saved connections from {:?}",
                    saved.connections.len(),
                    path
                );
                saved.connections
            }
            Err(e) => {
                warn!("Failed to parse connections file: {}", e);
                Vec::new()
            }
        },
        Err(_) => Vec::new(), // File doesn't exist yet, that's fine
    }
}

/// Manages multiple Cassandra connections
pub struct ConnectionManager {
    connections: Arc<RwLock<HashMap<Uuid, Arc<CassandraConnection>>>>,
    configs: Arc<RwLock<Vec<ConnectionConfig>>>,
    active_connection_id: Arc<RwLock<Option<Uuid>>>,
}

impl ConnectionManager {
    /// Create a new connection manager, loading any saved connections from disk
    pub fn new() -> Self {
        let saved_configs = load_persisted_configs();
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            configs: Arc::new(RwLock::new(saved_configs)),
            active_connection_id: Arc::new(RwLock::new(None)),
        }
    }

    /// Add a new connection configuration
    pub async fn add_config(&self, config: ConnectionConfig) -> Result<Uuid> {
        config.validate()?;

        let id = config.id;
        let mut configs = self.configs.write().await;

        // Check for duplicate names
        if configs.iter().any(|c| c.name == config.name && c.id != id) {
            return Err(MagdaError::validation(format!(
                "Connection with name '{}' already exists",
                config.name
            )));
        }

        configs.push(config);
        persist_configs(&configs);
        info!("Added connection configuration: {}", id);

        Ok(id)
    }

    /// Remove a connection configuration
    pub async fn remove_config(&self, id: Uuid) -> Result<()> {
        // Disconnect if active
        self.disconnect(id).await?;

        let mut configs = self.configs.write().await;
        configs.retain(|c| c.id != id);
        persist_configs(&configs);

        info!("Removed connection configuration: {}", id);
        Ok(())
    }

    /// Get all connection configurations
    pub async fn get_configs(&self) -> Vec<ConnectionConfig> {
        self.configs.read().await.clone()
    }

    /// Get a specific connection configuration
    pub async fn get_config(&self, id: Uuid) -> Option<ConnectionConfig> {
        self.configs
            .read()
            .await
            .iter()
            .find(|c| c.id == id)
            .cloned()
    }

    /// Connect to a Cassandra cluster
    pub async fn connect(&self, id: Uuid) -> Result<()> {
        let config = self
            .get_config(id)
            .await
            .ok_or_else(|| MagdaError::validation("Connection configuration not found"))?;

        info!(
            "Connecting to {} ({}:{})",
            config.name, config.host, config.port
        );

        let connection = CassandraConnection::connect(config.clone()).await?;

        // Test the connection
        connection.test().await?;

        let mut connections = self.connections.write().await;
        connections.insert(id, Arc::new(connection));

        // Always set as active connection when connecting
        let mut active = self.active_connection_id.write().await;
        *active = Some(id);
        debug!("Set {} as active connection", id);

        info!("Successfully connected to {}", config.name);
        Ok(())
    }

    /// Disconnect from a Cassandra cluster
    pub async fn disconnect(&self, id: Uuid) -> Result<()> {
        let mut connections = self.connections.write().await;

        if connections.remove(&id).is_some() {
            info!("Disconnected from connection: {}", id);

            // If this was the active connection, clear it
            let mut active = self.active_connection_id.write().await;
            if *active == Some(id) {
                *active = None;
                debug!("Cleared active connection");
            }
        }

        Ok(())
    }

    /// Get an active connection
    pub async fn get_connection(&self, id: Uuid) -> Option<Arc<CassandraConnection>> {
        self.connections.read().await.get(&id).cloned()
    }

    /// Get the current active connection
    pub async fn get_active_connection(&self) -> Option<Arc<CassandraConnection>> {
        let active_id = *self.active_connection_id.read().await;

        if let Some(id) = active_id {
            self.get_connection(id).await
        } else {
            None
        }
    }

    /// Set the active connection
    pub async fn set_active_connection(&self, id: Uuid) -> Result<()> {
        let connections = self.connections.read().await;

        if !connections.contains_key(&id) {
            return Err(MagdaError::validation("Connection not found or not active"));
        }

        let mut active = self.active_connection_id.write().await;
        *active = Some(id);

        debug!("Set active connection to: {}", id);
        Ok(())
    }

    /// Test a connection configuration without establishing a persistent connection
    pub async fn test_connection(config: &ConnectionConfig) -> Result<()> {
        info!("Testing connection to {}:{}", config.host, config.port);

        let connection = CassandraConnection::connect(config.clone()).await?;
        connection.test().await?;

        info!("Connection test successful");
        Ok(())
    }

    /// Get all active connections
    pub async fn get_active_connections(&self) -> Vec<Arc<CassandraConnection>> {
        self.connections.read().await.values().cloned().collect()
    }

    /// Check if a connection is active
    pub async fn is_connected(&self, id: Uuid) -> bool {
        self.connections.read().await.contains_key(&id)
    }

    /// Disconnect all connections
    pub async fn disconnect_all(&self) -> Result<()> {
        let ids: Vec<Uuid> = self.connections.read().await.keys().cloned().collect();

        for id in ids {
            if let Err(e) = self.disconnect(id).await {
                warn!("Failed to disconnect {}: {}", id, e);
            }
        }

        Ok(())
    }
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a manager without loading from disk (for isolated tests)
    fn new_in_memory() -> ConnectionManager {
        ConnectionManager {
            connections: Arc::new(RwLock::new(HashMap::new())),
            configs: Arc::new(RwLock::new(Vec::new())),
            active_connection_id: Arc::new(RwLock::new(None)),
        }
    }

    #[tokio::test]
    async fn test_connection_manager_config_operations() {
        let manager = new_in_memory();

        let config = ConnectionConfig::new("Test Connection", "localhost");
        let id = manager.add_config(config.clone()).await.unwrap();

        let configs = manager.get_configs().await;
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].id, id);

        let retrieved = manager.get_config(id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test Connection");

        manager.remove_config(id).await.unwrap();
        assert_eq!(manager.get_configs().await.len(), 0);
    }

    #[tokio::test]
    async fn test_duplicate_connection_names() {
        let manager = new_in_memory();

        let config1 = ConnectionConfig::new("Test", "localhost");
        manager.add_config(config1).await.unwrap();

        let config2 = ConnectionConfig::new("Test", "127.0.0.1");
        let result = manager.add_config(config2).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }
}
