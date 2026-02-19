use dioxus::prelude::*;

mod cassandra;
mod config;
mod connection;
mod cql_tokenizer;
mod error;
mod state;
mod components;

use crate::state::AppState;

const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    // Initialize logging
    init_logger();

    tracing::info!("Starting Magda - Cassandra Desktop Client");

    // Set macOS dock icon before launching the window
    #[cfg(target_os = "macos")]
    set_macos_dock_icon();

    // Launch the desktop application with window icon
    LaunchBuilder::new()
        .with_cfg(
            dioxus::desktop::Config::new().with_window(
                dioxus::desktop::WindowBuilder::new()
                    .with_title("Magda")
                    .with_window_icon(load_window_icon()),
            ),
        )
        .launch(App);
}

/// Load the app icon from the embedded PNG for the desktop window (Linux/Windows).
fn load_window_icon() -> Option<dioxus::desktop::tao::window::Icon> {
    let png_bytes = include_bytes!("../assets/icon.png");
    let img = image::load_from_memory(png_bytes).ok()?.into_rgba8();
    let (w, h) = img.dimensions();
    dioxus::desktop::tao::window::Icon::from_rgba(img.into_raw(), w, h).ok()
}

/// Set the macOS dock icon via NSApplication.setApplicationIconImage.
#[cfg(target_os = "macos")]
fn set_macos_dock_icon() {
    use objc2::AnyThread;
    use objc2_app_kit::{NSApplication, NSImage};
    use objc2_foundation::NSData;

    let png_bytes = include_bytes!("../assets/icon.png");
    unsafe {
        let data = NSData::with_bytes(png_bytes);
        let image = NSImage::initWithData(NSImage::alloc(), &data);
        if let Some(image) = image {
            let app = NSApplication::sharedApplication(objc2::MainThreadMarker::new().unwrap());
            app.setApplicationIconImage(Some(&image));
        }
    }
}

#[component]
fn App() -> Element {
    // Initialize shared application state
    use_context_provider(|| Signal::new(AppState::new()));
    
    rsx! {
        // Global app resources
        document::Title { "Magda" }
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
