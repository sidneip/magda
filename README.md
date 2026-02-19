<div align="center">

# Magda

A modern, fast desktop client for Apache Cassandra — built entirely in Rust.

[![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Dioxus](https://img.shields.io/badge/Dioxus-0.6-blue?style=flat)](https://dioxuslabs.com/)
[![Cassandra](https://img.shields.io/badge/Apache%20Cassandra-1287B1?style=flat&logo=apache-cassandra&logoColor=white)](https://cassandra.apache.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

</div>

---

<!-- TODO: Replace with actual screenshot after UI polish -->
<!-- ![Magda Screenshot](docs/screenshots/query-results.png) -->

## About

Magda is a native desktop client for Apache Cassandra, designed for developers who need a fast, lightweight alternative to browser-based tools. Built with [Dioxus](https://dioxuslabs.com/) and Rust, it delivers native performance with a clean, modern interface.

## Features

- **CQL Query Editor** — Write and execute CQL queries with keyboard shortcuts (`Ctrl+Enter`)
- **Multiple Connections** — Manage and switch between multiple Cassandra clusters
- **Schema Browser** — Explore keyspaces, tables, and column definitions
- **Data Grid** — Browse table data with automatic pagination
- **Query Variables** — Define reusable `{{variables}}` that get substituted into queries
- **Query History** — Click any past query to load it back into the editor
- **Persistent Config** — Connections and variables are saved to disk across sessions

## Getting Started

### Prerequisites

- [Rust toolchain](https://rustup.rs/) (stable)
- [Dioxus CLI](https://dioxuslabs.com/learn/0.6/getting_started): `cargo install dioxus-cli`
- A running Apache Cassandra instance (default: `localhost:9042`)

### Run

```bash
cd desktop
dx serve
```

### Build

```bash
cargo build --release
```

## Architecture

```
magda/
├── desktop/              # Desktop app (primary target)
│   ├── assets/           # CSS styles
│   └── src/
│       ├── main.rs               # App entry point
│       ├── state.rs              # Global state (Dioxus Signals)
│       ├── cassandra.rs          # CQL driver integration (cdrs-tokio)
│       ├── connection/           # Connection manager + config persistence
│       ├── config.rs             # User preferences + variables (TOML)
│       └── components/           # UI components
│           ├── workspace.rs              # Tab-based main area
│           ├── query_editor.rs           # CQL editor with Ctrl+Enter
│           ├── data_grid.rs              # Results table with pagination
│           ├── schema_viewer.rs          # Keyspace & table schema browser
│           ├── sidebar.rs                # Connection tree + table list
│           ├── variables_panel.rs        # Query variable management
│           ├── connection_dialog.rs      # New connection modal
│           └── statusbar.rs              # Connection status indicator
├── web/                  # Web target (shares architecture)
├── mobile/               # Mobile target
├── ui/                   # Shared UI components
└── api/                  # Shared backend logic
```

### Key Technical Decisions

| Decision | Why |
|---|---|
| **Dioxus Signals** for reactive state | Simpler than Redux-like patterns, built into the framework |
| **`Arc<RwLock<>>`** connection manager | Thread-safe multi-connection support without global mutex |
| **cdrs-tokio** Cassandra driver | Pure Rust, async-native, no C dependencies |
| **TOML** for config persistence | Human-readable, easy to edit, standard in Rust ecosystem |

## Roadmap

- [ ] Syntax highlighting for CQL
- [ ] Export results to CSV / JSON
- [ ] Connection import / export
- [ ] Light theme
- [ ] Query autocomplete
- [ ] Saved queries / snippets

## License

MIT
