# notes-tui — Minimal Rust Notepad TUI

Single-buffer Notepad clone in the terminal. Autosaves to a hidden file, exports to `~/Downloads/notes/`, no open dialog.

## Run

```bash
cargo run
```

Linux + mouse-enabled terminal required.

## Keys

- Type / Arrows: edit + move
- Mouse click: move cursor, drag: select
- `Tab` / `Shift+Tab`: indent / dedent (4 spaces)
- `Ctrl+Shift+C` / `Ctrl+Shift+V`: copy / paste (system clipboard, falls back to internal)
- `Ctrl+Z` / `Ctrl+Shift+Z`: undo / redo
- `Ctrl+S`: Save As prompt — always to `~/Downloads/notes/<name>.txt`
- `Ctrl+N`: clear/new (view cleared, old autosave kept until next typing)
- `Ctrl+F`: find popup, `Enter` next
- `Ctrl+H`: replace popup, `Enter` replace-one, `Ctrl+A` replace-all, `Tab` switch field
- `Ctrl+Q`: quit (autosave retained)
- `Esc`: close popup / cancel

Wrap + line numbers are always on.

## Files

- Autosave: `~/.config/notes-tui/autosave.txt` (created on first edit, loaded on launch)
- Export: `~/Downloads/notes/` (auto-created, overwrite asks y/n)
- `src/storage.rs`: autosave + export + filename sanitize
- `src/find.rs`: literal unicode-safe find/replace
- `src/app.rs`: state machine + editing ops
- `src/ui.rs`: rendering + popups
- `src/main.rs`: terminal + event loop + mouse

## Test

```bash
cargo test
cargo build
```

11 unit tests: storage roundtrip, sanitize, overwrite guard, find positions/unicode/wrap, replace counts, clear + tab.
