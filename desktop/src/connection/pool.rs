use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};
use tracing::{debug, warn};

use super::CassandraConnection;
use crate::error::{MagdaError, Result};

/// Connection pool for managing multiple connections to the same cluster
pub struct ConnectionPool {
    connections: Arc<RwLock<Vec<Arc<CassandraConnection>>>>,
    semaphore: Arc<Semaphore>,
    max_connections: usize,
    min_connections: usize,
}

impl ConnectionPool {
    /// Create a new connection pool
    pub fn new(min_connections: usize, max_connections: usize) -> Self {
        assert!(min_connections <= max_connections);
        assert!(max_connections > 0);
        
        Self {
            connections: Arc::new(RwLock::new(Vec::with_capacity(max_connections))),
            semaphore: Arc::new(Semaphore::new(max_connections)),
            max_connections,
            min_connections,
        }
    }
    
    /// Add a connection to the pool
    pub async fn add_connection(&self, connection: Arc<CassandraConnection>) -> Result<()> {
        let mut connections = self.connections.write().await;
        
        if connections.len() >= self.max_connections {
            return Err(MagdaError::validation(
                format!("Connection pool is full (max: {})", self.max_connections)
            ));
        }
        
        connections.push(connection);
        debug!("Added connection to pool. Current size: {}", connections.len());
        
        Ok(())
    }
    
    /// Get a connection from the pool
    pub async fn get_connection(&self) -> Result<Arc<CassandraConnection>> {
        let _permit = self.semaphore.acquire().await
            .map_err(|_| MagdaError::connection("Failed to acquire connection permit"))?;
        
        let connections = self.connections.read().await;
        
        if connections.is_empty() {
            return Err(MagdaError::connection("No connections available in pool"));
        }
        
        // Simple round-robin selection
        // In a production system, this could be more sophisticated
        let index = 0; // TODO: implement proper load balancing
        
        Ok(connections[index].clone())
    }
    
    /// Remove unhealthy connections from the pool
    pub async fn cleanup(&self) -> Result<()> {
        let mut connections = self.connections.write().await;
        let initial_count = connections.len();
        
        // Test each connection and keep only healthy ones
        let mut healthy_connections = Vec::new();
        
        for conn in connections.iter() {
            if conn.test().await.is_ok() {
                healthy_connections.push(conn.clone());
            } else {
                warn!("Removing unhealthy connection from pool");
            }
        }
        
        *connections = healthy_connections;
        
        let removed = initial_count - connections.len();
        if removed > 0 {
            debug!("Removed {} unhealthy connections from pool", removed);
        }
        
        Ok(())
    }
    
    /// Get the current pool size
    pub async fn size(&self) -> usize {
        self.connections.read().await.len()
    }
    
    /// Check if the pool needs more connections
    pub async fn needs_connections(&self) -> bool {
        self.size().await < self.min_connections
    }
    
    /// Clear all connections from the pool
    pub async fn clear(&self) -> Result<()> {
        let mut connections = self.connections.write().await;
        connections.clear();
        debug!("Cleared all connections from pool");
        Ok(())
    }
}

impl Default for ConnectionPool {
    fn default() -> Self {
        Self::new(1, 10)
    }
}