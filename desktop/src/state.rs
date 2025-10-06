use dioxus::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

use crate::connection::ConnectionManager;

/// Global application state
#[derive(Clone)]
pub struct AppState {
    pub connection_manager: Arc<ConnectionManager>,
    pub query_history: Signal<Vec<QueryHistoryItem>>,
    pub active_tab: Signal<ActiveTab>,
    pub sidebar_visible: Signal<bool>,
    pub theme: Signal<Theme>,
    pub selected_table: Signal<Option<String>>,
}

impl AppState {
    /// Create a new application state
    pub fn new() -> Self {
        Self {
            connection_manager: Arc::new(ConnectionManager::new()),
            query_history: Signal::new(Vec::new()),
            active_tab: Signal::new(ActiveTab::Query),
            sidebar_visible: Signal::new(true),
            theme: Signal::new(Theme::Dark),
            selected_table: Signal::new(None),
        }
    }
    
    /// Add a query to the history (used with Signal's clone pattern)
    pub fn add_to_history(mut query_history: Signal<Vec<QueryHistoryItem>>, query: String, success: bool, execution_time_ms: u64) {
        let item = QueryHistoryItem {
            id: Uuid::new_v4(),
            query,
            success,
            execution_time_ms,
            executed_at: chrono::Utc::now(),
        };
        
        let mut history = query_history.write();
        history.push(item);
        
        // Keep only the last 100 queries
        if history.len() > 100 {
            history.remove(0);
        }
    }
    
    /// Toggle sidebar visibility
    pub fn toggle_sidebar(mut sidebar_visible: Signal<bool>) {
        let current = *sidebar_visible.read();
        *sidebar_visible.write() = !current;
    }
    
    /// Switch theme
    pub fn toggle_theme(mut theme: Signal<Theme>) {
        let current_theme = *theme.read();
        let new_theme = match current_theme {
            Theme::Light => Theme::Dark,
            Theme::Dark => Theme::Light,
        };
        *theme.write() = new_theme;
    }
}

/// Query history item
#[derive(Clone, Debug)]
pub struct QueryHistoryItem {
    pub id: Uuid,
    pub query: String,
    pub success: bool,
    pub execution_time_ms: u64,
    pub executed_at: chrono::DateTime<chrono::Utc>,
}

/// Active tab in the main workspace
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ActiveTab {
    Query,
    Schema,
    Data,
    History,
}

/// Application theme
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Theme {
    Light,
    Dark,
}

impl Theme {
    /// Get CSS class name for the theme
    pub fn class_name(&self) -> &'static str {
        match self {
            Theme::Light => "theme-light",
            Theme::Dark => "theme-dark",
        }
    }
    
    /// Get theme colors
    pub fn colors(&self) -> ThemeColors {
        match self {
            Theme::Light => ThemeColors {
                background: "#ffffff",
                surface: "#f5f5f5",
                primary: "#007AFF",
                text_primary: "#000000",
                text_secondary: "#666666",
                border: "#e0e0e0",
                success: "#34C759",
                warning: "#FF9500",
                error: "#FF3B30",
            },
            Theme::Dark => ThemeColors {
                background: "#1a1a1a",
                surface: "#2a2a2a",
                primary: "#007AFF",
                text_primary: "#ffffff",
                text_secondary: "#8E8E93",
                border: "#3a3a3a",
                success: "#34C759",
                warning: "#FF9500",
                error: "#FF3B30",
            },
        }
    }
}

/// Theme color palette
#[derive(Clone, Debug)]
pub struct ThemeColors {
    pub background: &'static str,
    pub surface: &'static str,
    pub primary: &'static str,
    pub text_primary: &'static str,
    pub text_secondary: &'static str,
    pub border: &'static str,
    pub success: &'static str,
    pub warning: &'static str,
    pub error: &'static str,
}

/// Query execution state
#[derive(Clone, Debug)]
pub struct QueryExecution {
    pub is_running: bool,
    pub current_query: Option<String>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for QueryExecution {
    fn default() -> Self {
        Self {
            is_running: false,
            current_query: None,
            started_at: None,
        }
    }
}