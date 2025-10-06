use dioxus::prelude::*;

mod cassandra;
mod config;
mod connection;
mod error;
mod state;
mod components;

use crate::state::AppState;

const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    // Initialize logging
    init_logger();
    
    tracing::info!("Starting Magda - Cassandra Desktop Client");
    
    // Launch the desktop application
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    // Initialize shared application state
    use_context_provider(|| Signal::new(AppState::new()));
    
    rsx! {
        // Global app resources
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        
        div {
            class: "app-container",
            
            // Main layout with sidebar and content area
            div {
                class: "main-layout",
                
                // Sidebar with connection explorer
                components::sidebar::Sidebar {}
                
                // Main content area
                div {
                    class: "content-area",
                    
                    // Query editor and results
                    components::workspace::Workspace {}
                }
            }
            
            // Status bar at the bottom
            components::statusbar::StatusBar {}
        }
    }
}

/// Initialize the application logger with environment-based configuration
fn init_logger() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
    
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "magda_desktop=debug,cdrs_tokio=info,warn".into()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
        )
        .init();
}
