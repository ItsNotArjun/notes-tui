# Requirements: notes-tui — Minimal Rust Notepad TUI

## Goal
Build a minimal Rust-based TUI note-taking app that behaves like Windows Notepad for a single running buffer: type to edit with autosave persistence across restarts, explicitly export via Save As, and clear to a fresh buffer.

## Context
- **Type**: greenfield (no Cargo.toml / markers in `/home/arjunthakur/data/src/projects`; new folder `notes-tui/`)
- **Stack**: Rust + ratatui + crossterm (ratatui-textarea expected for editing)
- **Source**: user feature request interview, Oct 2026

## Scope
### In Scope
- Single editable text buffer (plain .txt, UTF-8)
- Hidden implicit autosave file, loaded on every launch
- Explicit Save As via Ctrl+S to fixed dir `~/Downloads/notes/` with filename prompt
- Clear / new buffer via Ctrl+N (corrected from Ctrl+M); cleared view keeps old autosave file until next typing overwrites it
- Editing:
  - Arrow-key cursor movement
  - Mouse click moves cursor, drag selects (always on)
  - Clipboard Ctrl+Shift+C / Ctrl+Shift+V
  - Tab / Shift+Tab indentation
  - Undo Ctrl+Z, Redo Ctrl+Shift+Z
  - Line wrap ON by default
  - Line numbers ON by default
  - Find Ctrl+F, Replace Ctrl+H with popup bar (standard keys)
- Quit via Ctrl+Q (autosave retained)
- Status hints for keys, save state, filename
- Auto-create `~/Downloads/notes/` if missing; confirm on filename overwrite
- Unit tests for core logic

### Out of Scope
- Open existing files (no Ctrl+O, no CLI-arg open, no file browser open)
- Formatting, markdown preview, syntax highlight, images, tabs, multi-note library
- Cloud sync, config UI, cross-platform support beyond Linux
- Full file-browser save UI (Save As is path prompt + filename only)

## Constraints
- Rust + ratatui + crossterm
- Linux only, xterm-compatible terminal with mouse support required
- Fixed export dir: `~/Downloads/notes/`
- Hidden autosave: implicit file not directly user-visible (exact path TBD, e.g. `~/.config/notes-tui/scratch.txt`)
- Keys locked:
  - Ctrl+N clear/new, Ctrl+S save-as, Ctrl+Q quit
  - Ctrl+F find, Ctrl+H replace
  - Ctrl+Z undo, Ctrl+Shift+Z redo
  - Ctrl+Shift+C/V copy/paste
  - Arrows, Tab/Shift+Tab
- Plain text UTF-8 only

## Acceptance Criteria
- [ ] `cargo build` succeeds with no warnings on Linux
- [ ] `cargo test` passes — unit tests cover buffer edit, autosave restore, Save As naming, Ctrl+N clear semantics, find/replace logic
- [ ] Relaunch restores last typed buffer from hidden autosave
- [ ] Ctrl+S prompts for filename and writes `<name>.txt` to `~/Downloads/notes/`, creating dir if needed
- [ ] Overwrite of existing name prompts for confirm
- [ ] Ctrl+N clears view; autosave file not wiped until next keystroke
- [ ] Mouse drag selects, Ctrl+Shift+C/V copies/pastes, Tab indents
- [ ] Wrap + line numbers visible by default, find bar finds/jumps and replace works

## Edge Cases & Open Questions
- Hidden autosave exact path — propose `~/.config/notes-tui/autosave.txt`
- Save-as: enforce `.txt` extension? Sanitize filename? What if `~/Downloads/notes` not writable?
- Empty buffer save — allow or block with warning?
- Very large paste / large file performance — cap or stream?
- Unicode graphemes / wide chars in wrap + selection — use textarea defaults?
- Terminal without mouse — fallback to keyboard-only selection?
- Ctrl+Q with no changes — quit immediately vs confirm?

## Interview Summary
Single-file Notepad model confirmed over multi-note library. Autosave is hidden and implicit; explicit save always goes to `~/Downloads/notes` with name prompt. Editing set includes mouse selection, Ctrl+Shift clipboard, Tab indent, wrap, numbers, find/replace. Stack chosen as ratatui + crossterm after pros/cons. Keys corrected to Ctrl+N for clear. Open explicitly excluded. Save UI is filename prompt (no browser). Platform Linux only. Verification via unit tests. View defaults all-on. Edge handling: auto-create dir, overwrite prompt, UTF-8.

---
Ambiguity Score: 0.27/1.0
Rounds: 14
