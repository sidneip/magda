# Design: CSV Export + Saved Queries

## CSV Export
- Export button on results header triggers native save dialog (rfd crate)
- `export_to_csv()` serializes QueryResult to RFC 4180 CSV
- Works in both Data Grid and Query Results views

## Saved Queries
- Sidebar section below Tables
- `SavedQuery { id, name, query }` persisted to `~/.config/Magda/saved_queries.toml`
- "Save" button in query toolbar opens inline name input
- Click saved query loads into editor + switches to Query tab
- Delete button per entry
