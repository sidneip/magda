use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tracing::{debug, info};

use crate::connection::ConnectionConfig;
use crate::error::{MagdaError, Result};
use crate::state::{QueryVariable, SavedQuery};

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub connections: Vec<ConnectionConfig>,
    pub preferences: UserPreferences,
    pub recent_queries: Vec<String>,
}

impl AppConfig {
    /// Create new configuration with defaults
    pub fn new() -> Self {
        Self {
            connections: Vec::new(),
            preferences: UserPreferences::default(),
            recent_queries: Vec::new(),
        }
    }
    
    /// Load configuration from disk
    pub fn load() -> Result<Self> {
        let config_path = Self::config_file_path()?;
        
        if !config_path.exists() {
            info!("Config file not found, creating default configuration");
            let config = Self::new();
            config.save()?;
            return Ok(config);
        }
        
        let content = fs::read_to_string(&config_path)
            .map_err(|e| MagdaError::ConfigError(format!("Failed to read config: {}", e)))?;
        
        let config: Self = toml::from_str(&content)
            .map_err(|e| MagdaError::ConfigError(format!("Failed to parse config: {}", e)))?;
        
        debug!("Loaded configuration from {:?}", config_path);
        Ok(config)
    }
    
    /// Save configuration to disk
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_file_path()?;
        
        // Ensure directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| MagdaError::ConfigError(format!("Failed to create config directory: {}", e)))?;
        }
        
        let content = toml::to_string_pretty(self)
            .map_err(|e| MagdaError::ConfigError(format!("Failed to serialize config: {}", e)))?;
        
        fs::write(&config_path, content)
            .map_err(|e| MagdaError::ConfigError(format!("Failed to write config: {}", e)))?;
        
        debug!("Saved configuration to {:?}", config_path);
        Ok(())
    }
    
    /// Get the configuration file path
    fn config_file_path() -> Result<PathBuf> {
        let proj_dirs = ProjectDirs::from("com", "magda", "Magda")
            .ok_or_else(|| MagdaError::ConfigError("Failed to determine config directory".to_string()))?;
        
        Ok(proj_dirs.config_dir().join("config.toml"))
    }
    
    /// Add a connection configuration
    pub fn add_connection(&mut self, connection: ConnectionConfig) {
        self.connections.push(connection);
    }
    
    /// Remove a connection configuration
    pub fn remove_connection(&mut self, id: uuid::Uuid) {
        self.connections.retain(|c| c.id != id);
    }
    
    /// Update a connection configuration
    pub fn update_connection(&mut self, connection: ConnectionConfig) {
        if let Some(existing) = self.connections.iter_mut().find(|c| c.id == connection.id) {
            *existing = connection;
        }
    }
    
    /// Add a query to recent queries
    pub fn add_recent_query(&mut self, query: String) {
        // Remove if already exists to move to front
        self.recent_queries.retain(|q| q != &query);
        
        // Add to front
        self.recent_queries.insert(0, query);
        
        // Keep only last 50 queries
        if self.recent_queries.len() > 50 {
            self.recent_queries.truncate(50);
        }
    }
}

/// Wrapper for TOML serialization of variables
#[derive(Debug, Serialize, Deserialize)]
struct VariablesFile {
    variables: Vec<QueryVariable>,
}

fn variables_file_path() -> Option<PathBuf> {
    ProjectDirs::from("com", "magda", "Magda")
        .map(|dirs| dirs.config_dir().join("variables.toml"))
}

/// Load query variables from disk, returning an empty vec on any error.
pub fn load_variables() -> Vec<QueryVariable> {
    let Some(path) = variables_file_path() else {
        return Vec::new();
    };
    let Ok(content) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    toml::from_str::<VariablesFile>(&content)
        .map(|f| f.variables)
        .unwrap_or_default()
}

/// Save query variables to disk.
pub fn save_variables(vars: &[QueryVariable]) {
    let Some(path) = variables_file_path() else {
        tracing::warn!("Could not determine config directory for variables");
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let file = VariablesFile {
        variables: vars.to_vec(),
    };
    match toml::to_string_pretty(&file) {
        Ok(content) => {
            if let Err(e) = fs::write(&path, content) {
                tracing::warn!("Failed to save variables: {}", e);
            }
        }
        Err(e) => tracing::warn!("Failed to serialize variables: {}", e),
    }
}

/// Wrapper for TOML serialization of saved queries
#[derive(Debug, Serialize, Deserialize)]
struct SavedQueriesFile {
    queries: Vec<SavedQuery>,
}

fn saved_queries_file_path() -> Option<PathBuf> {
    ProjectDirs::from("com", "magda", "Magda")
        .map(|dirs| dirs.config_dir().join("saved_queries.toml"))
}

/// Load saved queries from disk.
pub fn load_saved_queries() -> Vec<SavedQuery> {
    let Some(path) = saved_queries_file_path() else {
        return Vec::new();
    };
    let Ok(content) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    toml::from_str::<SavedQueriesFile>(&content)
        .map(|f| f.queries)
        .unwrap_or_default()
}

/// Save queries to disk.
pub fn save_saved_queries(queries: &[SavedQuery]) {
    let Some(path) = saved_queries_file_path() else {
        tracing::warn!("Could not determine config directory for saved queries");
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let file = SavedQueriesFile {
        queries: queries.to_vec(),
    };
    match toml::to_string_pretty(&file) {
        Ok(content) => {
            if let Err(e) = fs::write(&path, content) {
                tracing::warn!("Failed to save queries: {}", e);
            }
        }
        Err(e) => tracing::warn!("Failed to serialize saved queries: {}", e),
    }
}

/// User preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub theme: String,
    pub font_size: u16,
    pub font_family: String,
    pub show_line_numbers: bool,
    pub word_wrap: bool,
    pub auto_complete: bool,
    pub auto_save: bool,
    pub query_timeout_seconds: u64,
    pub max_rows_to_fetch: usize,
    pub sidebar_width: u32,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            font_size: 14,
            font_family: "JetBrains Mono".to_string(),
            show_line_numbers: true,
            word_wrap: false,
            auto_complete: true,
            auto_save: true,
            query_timeout_seconds: 30,
            max_rows_to_fetch: 1000,
            sidebar_width: 280,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_config_serialization() {
        let config = AppConfig::new();
        let toml_str = toml::to_string(&config).unwrap();
        let deserialized: AppConfig = toml::from_str(&toml_str).unwrap();
        
        assert_eq!(config.connections.len(), deserialized.connections.len());
        assert_eq!(config.preferences.theme, deserialized.preferences.theme);
    }
    
    #[test]
    fn test_recent_queries() {
        let mut config = AppConfig::new();
        
        config.add_recent_query("SELECT * FROM users".to_string());
        assert_eq!(config.recent_queries.len(), 1);
        
        config.add_recent_query("SELECT * FROM posts".to_string());
        assert_eq!(config.recent_queries.len(), 2);
        
        // Adding duplicate should move to front
        config.add_recent_query("SELECT * FROM users".to_string());
        assert_eq!(config.recent_queries.len(), 2);
        assert_eq!(config.recent_queries[0], "SELECT * FROM users");
    }
}