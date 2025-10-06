use thiserror::Error;

/// Main error type for the Magda application
#[derive(Debug, Error)]
pub enum MagdaError {
    #[error("Connection failed: {0}")]
    ConnectionError(String),
    
    #[error("Query execution failed: {0}")]
    QueryError(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Schema inspection failed: {0}")]
    SchemaError(String),
    
    #[error("Authentication failed: {0}")]
    AuthError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("Invalid input: {0}")]
    ValidationError(String),
    
    #[error("Database driver error: {0}")]
    DriverError(String),
    
    #[error("Unknown error occurred")]
    Unknown,
}

impl From<cdrs_tokio::error::Error> for MagdaError {
    fn from(error: cdrs_tokio::error::Error) -> Self {
        MagdaError::QueryError(error.to_string())
    }
}

/// Result type alias for Magda operations
pub type Result<T> = std::result::Result<T, MagdaError>;

impl MagdaError {
    /// Create a connection error with a custom message
    pub fn connection(msg: impl Into<String>) -> Self {
        Self::ConnectionError(msg.into())
    }
    
    /// Create a query error with a custom message
    pub fn query(msg: impl Into<String>) -> Self {
        Self::QueryError(msg.into())
    }
    
    /// Create a validation error with a custom message
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::ValidationError(msg.into())
    }
    
    /// Check if this error is recoverable
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::ConnectionError(_) | Self::QueryError(_) | Self::AuthError(_)
        )
    }
    
    /// Get a user-friendly error message
    pub fn user_message(&self) -> String {
        match self {
            Self::ConnectionError(_) => "Unable to connect to the database. Please check your connection settings.".to_string(),
            Self::QueryError(_) => "Query execution failed. Please check your CQL syntax.".to_string(),
            Self::AuthError(_) => "Authentication failed. Please verify your credentials.".to_string(),
            Self::ValidationError(msg) => format!("Invalid input: {}", msg),
            _ => "An unexpected error occurred. Please try again.".to_string(),
        }
    }
}