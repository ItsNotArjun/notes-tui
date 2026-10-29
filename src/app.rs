use std::path::PathBuf;

use ratatui::style::{Color, Style};
use ratatui_textarea::{CursorMove, TextArea};

use crate::find;
use crate::storage;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Editing,
    SavePrompt,
    OverwriteConfirm(PathBuf),
    FindPrompt,
    ReplacePrompt,
}

pub struct App {
    pub textarea: TextArea<'static>,
    pub mode: Mode,
    pub save_input: String,
    pub find_input: String,
    pub replace_input: String,
    /// In replace popup, true = editing find field, false = editing replace field.
    pub replace_focus_find: bool,
    pub message: String,
    pub autosave_path: PathBuf,
    pub export_dir: PathBuf,
    /// Set after Ctrl+N: view is cleared but autosave file is kept until next typing.
    pub clear_deferred: bool,
    pub should_quit: bool,
    pub last_saved_export: Option<PathBuf>,
}

impl App {
    pub fn new(autosave_path: PathBuf, export_dir: PathBuf, initial: Vec<String>) -> Self {
        let mut textarea = TextArea::new(initial);
        textarea.set_tab_length(4);
        textarea.set_hard_tab_indent(false);
        textarea.set_line_number_style(Style::default().fg(Color::Green));
        textarea.set_cursor_line_style(Style::default());
        // Hide widget fake block cursor — use real terminal I-beam instead.
        textarea.set_cursor_style(Style::default());
        textarea.set_selection_style(Style::default().bg(Color::Blue).fg(Color::White));
        textarea.set_wrap_mode(ratatui_textarea::WrapMode::WordOrGlyph);
        Self {
            textarea,
            mode: Mode::Editing,
            save_input: String::new(),
            find_input: String::new(),
            replace_input: String::new(),
            replace_focus_find: true,
            message: String::from("Ctrl+S save  Ctrl+N new  Ctrl+F find  Ctrl+H replace  Ctrl+Q quit"),
            autosave_path,
            export_dir,
            clear_deferred: false,
            should_quit: false,
            last_saved_export: None,
        }
    }

    pub fn lines(&self) -> Vec<String> {
        self.textarea.lines().to_vec()
    }

    /// Called after any buffer mutation. Persists autosave unless in deferred-clear state.
    /// First typing after a clear exits deferred mode and then saves.
    pub fn mark_edited(&mut self) {
        if self.clear_deferred {
            self.clear_deferred = false;
        }
        if let Err(e) = storage::save_autosave(&self.autosave_path, &self.lines()) {
            self.message = format!("autosave failed: {e}");
        }
    }

    pub fn persist_autosave_if_allowed(&mut self) {
        if self.clear_deferred {
            return; // keep old autosave until next typing
        }
        if let Err(e) = storage::save_autosave(&self.autosave_path, &self.lines()) {
            self.message = format!("autosave failed: {e}");
        }
    }

    pub fn clear_buffer(&mut self) {
        // TextArea::select_all + cut would pollute yank; use move + delete via clear-like rebuild.
        // Simplest reliable clear that preserves undo: select all then type empty.
        self.textarea.select_all();
        self.textarea.cut();
        self.textarea.cancel_selection();
        self.textarea.move_cursor(CursorMove::Top);
        self.clear_deferred = true;
        self.message = "Cleared — autosave kept until next typing. Ctrl+S to export.".to_string();
    }

    pub fn copy_to_system(&mut self) {
        let selected = self.textarea.yank_text();
        if selected.is_empty() {
            self.message = "Nothing selected to copy".to_string();
            return;
        }
        match arboard::Clipboard::new() {
            Ok(mut cb) => match cb.set_text(selected) {
                Ok(_) => self.message = "Copied to system clipboard".to_string(),
                Err(e) => self.message = format!("Clipboard copy failed, kept internal: {e}"),
            },
            Err(e) => self.message = format!("Clipboard unavailable, kept internal: {e}"),
        }
    }

    pub fn paste_from_system(&mut self) {
        let text = match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
            Ok(t) => t,
            Err(_) => {
                // Fallback to internal yank (TextArea copy buffer)
                let yank = self.textarea.yank_text();
                if yank.is_empty() {
                    self.message = "Clipboard empty".to_string();
                    return;
                }
                yank
            }
        };
        // Normalize newlines, insert
        let normalized = text.replace("\r\n", "\n");
        self.textarea.insert_str(normalized);
        self.mark_edited();
    }

    pub fn insert_tab(&mut self) {
        self.textarea.insert_str("    ");
        self.mark_edited();
    }

    pub fn dedent_current_line(&mut self) {
        let cursor = self.textarea.cursor();
        let row = cursor.0;
        let col = cursor.1;
        let lines = self.lines();
        let Some(line) = lines.get(row) else { return };
        let mut remove_chars = 0;
        let mut remove_bytes = 0;
        for c in line.chars() {
            if remove_chars >= 4 {
                break;
            }
            if c == ' ' {
                remove_chars += 1;
                remove_bytes += 1;
            } else if c == '\t' {
                remove_chars += 4; // treat tab as one indent level
                remove_bytes += c.len_utf8();
                break;
            } else {
                break;
            }
        }
        if remove_chars == 0 {
            return;
        }
        // Move to line head and delete forward
        self.textarea.cancel_selection();
        self.textarea.move_cursor(CursorMove::Jump(row as u16, 0));
        for _ in 0..line[..remove_bytes].chars().count() {
            self.textarea.delete_next_char();
        }
        let new_col = col.saturating_sub(remove_chars);
        self.textarea.move_cursor(CursorMove::Jump(row as u16, new_col as u16));
        self.mark_edited();
    }

    pub fn undo(&mut self) {
        if self.textarea.undo() {
            self.mark_edited();
        }
    }

    pub fn delete_word_backward(&mut self) {
        if self.textarea.delete_word() {
            self.mark_edited();
        }
    }

    pub fn redo(&mut self) {
        if self.textarea.redo() {
            self.mark_edited();
        }
    }

    // ---- Save flow ----

    pub fn start_save(&mut self) {
        self.save_input.clear();
        self.mode = Mode::SavePrompt;
        self.message = "Type filename, Enter to save to ~/Downloads/notes".to_string();
    }

    pub fn confirm_save(&mut self) {
        let name = self.save_input.clone();
        match storage::save_export(&self.export_dir, &name, &self.lines(), false) {
            Ok(path) => {
                self.last_saved_export = Some(path.clone());
                self.message = format!("Saved to {}", path.display());
                self.mode = Mode::Editing;
                self.save_input.clear();
            }
            Err(storage::SaveError::Exists(path)) => {
                self.mode = Mode::OverwriteConfirm(path);
                self.message = "File exists — overwrite? (y/n)".to_string();
            }
            Err(e) => {
                self.message = format!("Save failed: {e}");
            }
        }
    }

    pub fn confirm_overwrite(&mut self, yes: bool) {
        let Mode::OverwriteConfirm(path) = self.mode.clone() else {
            return;
        };
        if !yes {
            self.mode = Mode::SavePrompt;
            self.message = "Overwrite cancelled".to_string();
            return;
        }
        let filename = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        match storage::save_export(&self.export_dir, &filename, &self.lines(), true) {
            Ok(p) => {
                self.last_saved_export = Some(p.clone());
                self.message = format!("Overwrote {}", p.display());
                self.mode = Mode::Editing;
                self.save_input.clear();
            }
            Err(e) => {
                self.message = format!("Overwrite failed: {e}");
                self.mode = Mode::SavePrompt;
            }
        }
    }

    // ---- Find / replace ----

    pub fn start_find(&mut self) {
        self.mode = Mode::FindPrompt;
    }

    pub fn close_find(&mut self) {
        let _ = self.textarea.set_search_pattern("");
        self.mode = Mode::Editing;
        self.message = "Find closed".to_string();
    }

    pub fn close_replace(&mut self) {
        let _ = self.textarea.set_search_pattern("");
        self.mode = Mode::Editing;
        self.message = "Replace closed".to_string();
    }

    pub fn start_replace(&mut self) {
        self.replace_focus_find = true;
        self.mode = Mode::ReplacePrompt;
    }

    pub fn apply_find(&mut self, jump_next: bool) {
        let q = self.find_input.clone();
        if q.is_empty() {
            let _ = self.textarea.set_search_pattern("");
            self.message = "Find: type text, Enter for next".to_string();
            return;
        }
        // Use escaped literal for textarea regex highlight
        let pattern = regex::escape(&q);
        match self.textarea.set_search_pattern(&pattern) {
            Ok(_) => {
                let moved = if jump_next {
                    self.textarea.search_forward(false)
                } else {
                    // first jump: from current cursor
                    self.textarea.search_forward(true)
                        || self.textarea.search_forward(false)
                };
                if moved {
                    self.message = format!("Found `{q}`");
                } else {
                    // Fallback: wrap from top using our helper for accurate message
                    let cursor = self.textarea.cursor();
                    if let Some((r, c)) =
                        find::find_next(&self.lines(), &q, cursor.0, cursor.1, true)
                    {
                        self.textarea
                            .move_cursor(CursorMove::Jump(r as u16, c as u16));
                        self.message = format!("Found `{q}` (wrapped)");
                    } else {
                        self.message = format!("Not found: `{q}`");
                    }
                }
            }
            Err(e) => self.message = format!("Invalid search: {e}"),
        }
    }

    pub fn apply_replace_current(&mut self) {
        let q = self.find_input.clone();
        let r = self.replace_input.clone();
        if q.is_empty() {
            self.message = "Replace: find text is empty".to_string();
            return;
        }
        // Replace at/below cursor: find next occurrence, splice it.
        let cursor = self.textarea.cursor();
        let lines = self.lines();
        let Some((row, col_char)) = find::find_next(&lines, &q, cursor.0, cursor.1, true) else {
            self.message = format!("Not found: `{q}`");
            return;
        };
        let mut new_lines = lines;
        if let Some(line) = new_lines.get_mut(row) {
            let byte_start = line
                .char_indices()
                .nth(col_char)
                .map(|(b, _)| b)
                .unwrap_or(line.len());
            let q_bytes = q.len();
            if line[byte_start..].starts_with(&q) {
                line.replace_range(byte_start..byte_start + q_bytes, &r);
            } else {
                // Fallback for edge cases: literal replace first occurrence in line
                *line = line.replacen(&q, &r, 1);
            }
        }
        let new_col = col_char + r.chars().count();
        self.textarea = rebuild_textarea(std::mem::take(&mut self.textarea), &new_lines);
        self.textarea
            .move_cursor(CursorMove::Jump(row as u16, new_col as u16));
        self.mark_edited();
        self.message = format!("Replaced one `{q}` -> `{r}`");
    }

    pub fn apply_replace_all(&mut self) {
        let q = self.find_input.clone();
        let r = self.replace_input.clone();
        if q.is_empty() {
            self.message = "Replace: find text is empty".to_string();
            return;
        }
        let (out, n) = find::replace_all(&self.lines(), &q, &r);
        if n == 0 {
            self.message = format!("Not found: `{q}`");
            return;
        }
        self.textarea = rebuild_textarea(std::mem::take(&mut self.textarea), &out);
        self.mark_edited();
        self.message = format!("Replaced {n} occurrence(s)");
    }
}

/// Rebuild TextArea from lines while keeping visual config.
/// Undo history is reset — acceptable for replace-all (documented).
fn rebuild_textarea(old: TextArea<'static>, lines: &[String]) -> TextArea<'static> {
    let _ = old;
    let mut t = TextArea::new(lines.to_vec());
    t.set_tab_length(4);
    t.set_hard_tab_indent(false);
    t.set_line_number_style(ratatui::style::Style::default().fg(ratatui::style::Color::Green));
    t.set_cursor_line_style(ratatui::style::Style::default());
    t.set_cursor_style(ratatui::style::Style::default());
    t.set_selection_style(
        ratatui::style::Style::default()
            .bg(ratatui::style::Color::Blue)
            .fg(ratatui::style::Color::White),
    );
    t.set_wrap_mode(ratatui_textarea::WrapMode::WordOrGlyph);
    // Re-apply search highlight if any? Caller sets again on next find.
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_app() -> App {
        App::new(
            PathBuf::from("/tmp/notes-tui-test-autosave.txt"),
            PathBuf::from("/tmp/notes-tui-test-export"),
            vec!["hello".to_string()],
        )
    }

    #[test]
    fn clear_defers_autosave_flag() {
        let mut a = test_app();
        a.clear_buffer();
        assert!(a.clear_deferred);
        assert!(a.textarea.is_empty());
    }

    #[test]
    fn tab_inserts_spaces() {
        let mut a = test_app();
        a.insert_tab();
        assert!(a.lines()[0].contains("    "));
    }

    #[test]
    fn close_find_clears_highlight() {
        let mut a = test_app();
        a.find_input = "hello".to_string();
        a.apply_find(false);
        assert!(a.textarea.search_pattern().is_some());
        a.close_find();
        assert!(a.textarea.search_pattern().is_none());
    }

    #[test]
    fn empty_query_clears_highlight() {
        let mut a = test_app();
        a.find_input = "hello".to_string();
        a.apply_find(false);
        assert!(a.textarea.search_pattern().is_some());
        a.find_input.clear();
        a.apply_find(false);
        assert!(a.textarea.search_pattern().is_none());
    }

    #[test]
    fn ctrl_backspace_deletes_word() {
        let mut a = App::new(
            PathBuf::from("/tmp/notes-tui-test-autosave.txt"),
            PathBuf::from("/tmp/notes-tui-test-export"),
            vec!["hello world".to_string()],
        );
        a.textarea
            .move_cursor(CursorMove::Jump(0, 11));
        a.delete_word_backward();
        assert_eq!(a.lines(), vec!["hello ".to_string()]);
        a.delete_word_backward();
        assert_eq!(a.lines(), vec![String::new()]);
    }
}
