# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Magda is a modern desktop client for Apache Cassandra, built with Dioxus (Rust UI framework). The project is structured as a Cargo workspace with multiple platform targets (desktop, web, mobile) and shared components.

## Build & Development Commands

### Desktop Application (Primary Target)

```bash
# Navigate to desktop crate
cd desktop

# Run the desktop application
dx serve

# Run with specific logging
RUST_LOG=magda_desktop=debug,cdrs_tokio=info dx serve

# Build the desktop application
cargo build --release

# Run tests
cargo test

# Test Cassandra connection directly (without UI)
cargo run --bin test_connection
```

### Other Platform Targets

```bash
# Web platform
cd web && dx serve

# Mobile platform
cd mobile && dx serve
```

### Workspace-Level Commands

```bash
# Build all workspace members
cargo build --workspace

# Run tests across all crates
cargo test --workspace

# Check all code with Clippy (respects clippy.toml settings)
cargo clippy --workspace
```

## Architecture

### Workspace Structure

This is a Dioxus multi-platform workspace with:
- `desktop/` - Desktop application (primary development target)
- `web/` - Web application (shares UI structure)
- `mobile/` - Mobile application (shares UI structure)
- `ui/` - Shared UI components used across platforms
- `api/` - Shared backend/server logic (currently minimal, project focuses on desktop)

### Desktop Application Architecture

The desktop app is structured around a global application state and component-based UI:

**State Management** (desktop/src/state.rs):
- `AppState` - Global state with Dioxus `Signal`s
- Manages `ConnectionManager`, query history, active tabs, sidebar visibility, theme
- State access pattern: `use_context_provider` in main App, `use_context` in components

**Connection Layer** (desktop/src/connection/):
- `ConnectionManager` - Thread-safe manager for multiple Cassandra connections using `Arc<RwLock<>>` pattern
- `ConnectionConfig` - Serializable connection configuration with validation
- `CassandraConnection` - Wrapper for active Cassandra sessions
- Supports multiple simultaneous connections, single "active" connection for queries

**Cassandra Integration** (desktop/src/cassandra.rs):
- Uses `cdrs-tokio` driver (v8.1)
- `CassandraSession` - Wraps cdrs-tokio session with Arc for thread safety
- Core operations: `create_session`, `list_keyspaces`, `list_tables`, `execute_query`, `test_connection`
- Data conversion: `convert_cassandra_value` handles type mapping from Cassandra bytes to JSON values
- Query results returned as `QueryResult` with columns, rows (as JSON), execution time, row count

**Component Structure** (desktop/src/components/):
- `sidebar::Sidebar` - Connection explorer and navigation
- `workspace::Workspace` - Main content area (query editor + results)
- `query_editor::QueryEditor` - CQL query input
- `data_grid::DataGrid` - Results table viewer
- `schema_viewer::SchemaViewer` - Database schema inspector
- `connection_dialog::ConnectionDialog` - Connection configuration UI
- `statusbar::StatusBar` - Status information

**Error Handling** (desktop/src/error.rs):
- `MagdaError` - Central error enum using `thiserror`
- Includes user-friendly messages via `user_message()` method
- Implements `From<cdrs_tokio::error::Error>` for seamless driver error conversion

**Logging**:
- Default filter: `magda_desktop=debug,cdrs_tokio=info,warn`
- Configured in `main.rs` with `tracing-subscriber`
- Use `tracing::info!`, `tracing::debug!`, `tracing::warn!`, etc.

## Key Development Patterns

### Async Operations in Dioxus

When calling async Cassandra operations from UI components:

```rust
let connection_manager = use_context::<Signal<AppState>>().read().connection_manager.clone();

// Spawn async operations
spawn(async move {
    match connection_manager.connect(id).await {
        Ok(_) => { /* update UI state */ },
        Err(e) => { /* handle error */ }
    }
});
```

### Signal Usage

Following Dioxus signal patterns with the clippy.toml rules:
- Never hold `Signal::read()` or `Signal::write()` across await points
- This causes Clippy errors per `await-holding-invalid-types` configuration
- Read/write immediately, then drop the guard before awaiting

### Connection Manager Pattern

The `ConnectionManager` uses `Arc<RwLock<>>` internally for thread safety:
- Multiple readers OR single writer at a time
- Always clone the manager before spawning async tasks
- Use `await` on all async methods (connect, disconnect, get_config, etc.)

### Adding New CQL Operations

To add new Cassandra query operations:

1. Add async function to `desktop/src/cassandra.rs` that takes `&CassandraSession`
2. Add corresponding method to `CassandraConnection` in `desktop/src/connection/mod.rs`
3. Call via `ConnectionManager.get_active_connection()` from UI components
4. Handle result conversion for data types using pattern from `convert_cassandra_value`

### Testing Cassandra Integration

Use the test binary for quick iteration:

```bash
cargo run --bin test_connection
```

This connects to localhost:9042 and tests basic operations without launching the UI.

## Dependencies

Key dependencies and their versions:
- `dioxus = "0.6.0"` - UI framework
- `cdrs-tokio = "8.1"` - Cassandra driver (with "derive" feature)
- `tokio = "1"` - Async runtime (with "full" features)
- `serde = "1.0"` / `serde_json = "1.0"` - Serialization
- `uuid = "1.0"` - Connection and query IDs
- `chrono = "0.4"` - Timestamps
- `tracing` / `tracing-subscriber` - Logging
- `thiserror = "1.0"` - Error handling
- `anyhow = "1.0"` - Error context

## Configuration

- Clippy rules defined in `clippy.toml` - specifically prevents holding Dioxus signal references across await points
- Cargo workspace resolver = "2"
- Build profiles: `wasm-dev`, `server-dev`, `android-dev` (though desktop is primary target)

## Common Development Tasks

### Adding a New UI Component

1. Create new module file in `desktop/src/components/`
2. Add component function with `#[component]` attribute
3. Export from `desktop/src/components/mod.rs`
4. Access AppState via `use_context::<Signal<AppState>>()`

### Modifying Connection Configuration

Edit `ConnectionConfig` in `desktop/src/connection/mod.rs`. Remember to:
- Update validation in `validate()` method
- Update serialization if changing persistent fields
- Update `ConnectionDialog` UI component to expose new fields

### Adding Query History or Preferences

Persistent state can be added to:
- `desktop/src/config.rs` for user preferences
- Consider using `directories` crate for platform-specific config paths
