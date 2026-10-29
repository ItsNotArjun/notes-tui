mod app;
mod find;
mod storage;
mod ui;

use std::io;
use std::time::Duration;

use app::{App, Mode};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
    KeyboardEnhancementFlags, MouseButton, MouseEvent, MouseEventKind, PopKeyboardEnhancementFlags,
    PushKeyboardEnhancementFlags,
};
use ratatui::crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::crossterm::cursor::SetCursorStyle;
use ratatui::crossterm::execute;
use ratatui::layout::{Position, Rect};
use ratatui_textarea::{CursorMove, DataCursor, Input};

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("notes-tui — minimal Notepad TUI\n\nUsage: notes\n\nKeys:\n  Ctrl+S save to ~/Downloads/notes/  Ctrl+N new/clear\n  Ctrl+F find  Ctrl+H replace  Ctrl+Q quit\n  Tab indent  Ctrl+Z undo  Ctrl+Shift+Z redo\n  Ctrl+Shift+C/V copy/paste  Mouse drag select");
        return Ok(());
    }
    if args.iter().any(|a| a == "-V" || a == "--version") {
        println!("notes-tui {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let autosave_path = storage::autosave_path();
    let export_dir = storage::export_dir();
    let initial = storage::load_autosave(&autosave_path);
    let mut app = App::new(autosave_path, export_dir, initial);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    // Ask modern terminals (Kitty protocol) to disambiguate Ctrl+Backspace vs Ctrl+H.
    // Ignore errors — legacy terminals just won't get distinct codes.
    let _ = execute!(
        stdout,
        PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS
        )
    );
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    // Bar cursor while editing (steady, Notepad-like).
    let _ = execute!(terminal.backend_mut(), SetCursorStyle::SteadyBar);

    let mut editor_area = Rect::default();
    let res = run(&mut terminal, &mut app, &mut editor_area);

    disable_raw_mode()?;
    let _ = execute!(terminal.backend_mut(), PopKeyboardEnhancementFlags);
    let _ = execute!(
        terminal.backend_mut(),
        SetCursorStyle::DefaultUserShape
    );
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(e) = res {
        eprintln!("error: {e}");
    }
    Ok(())
}

fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    editor_area: &mut Rect,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app, editor_area))?;
        if app.should_quit {
            app.persist_autosave_if_allowed();
            break;
        }
        if !event::poll(Duration::from_millis(200))? {
            continue;
        }
        match event::read()? {
            Event::Key(k) => handle_key(app, k),
            Event::Mouse(m) => handle_mouse(app, editor_area, m),
            Event::Resize(_, _) => {}
            _ => {}
        }
    }
    Ok(())
}

fn ctrl_shift(key: &KeyEvent, c: char) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL)
        && key.modifiers.contains(KeyModifiers::SHIFT)
        && matches!(key.code, KeyCode::Char(ch) if ch.to_ascii_lowercase() == c)
}

fn ctrl(key: &KeyEvent, c: char) -> bool {
    key.modifiers == KeyModifiers::CONTROL
        && matches!(key.code, KeyCode::Char(ch) if ch.to_ascii_lowercase() == c)
}

fn handle_key(app: &mut App, key: KeyEvent) {
    match app.mode.clone() {
        Mode::Editing => handle_editing_key(app, key),
        Mode::SavePrompt => handle_save_prompt_key(app, key),
        Mode::OverwriteConfirm(_) => handle_overwrite_key(app, key),
        Mode::FindPrompt => handle_find_key(app, key),
        Mode::ReplacePrompt => handle_replace_key(app, key),
    }
}

fn handle_editing_key(app: &mut App, key: KeyEvent) {
    // Ctrl+Backspace must win over Ctrl+H on terminals that conflate them (0x08).
    // With Kitty DISAMBIGUATE flag they arrive distinct; without it, Backspace check
    // still catches the Backspace keycode variant.
    if matches!(key.code, KeyCode::Backspace)
        && key.modifiers.contains(KeyModifiers::CONTROL)
    {
        app.delete_word_backward();
        return;
    }
    // Ctrl+W is the classic terminal word-delete — alias for the same action.
    if ctrl(&key, 'w') {
        app.delete_word_backward();
        return;
    }
    // Custom global keys first
    if ctrl(&key, 'n') {
        app.clear_buffer();
        return;
    }
    if ctrl(&key, 's') {
        app.start_save();
        return;
    }
    if ctrl(&key, 'q') {
        app.should_quit = true;
        return;
    }
    if ctrl(&key, 'f') {
        app.start_find();
        return;
    }
    if ctrl(&key, 'h') {
        app.start_replace();
        return;
    }
    if ctrl(&key, 'z') && !key.modifiers.contains(KeyModifiers::SHIFT) {
        // Ctrl+Z (without shift) — undo. Note Ctrl+Shift+Z handled below.
        if !(key.modifiers.contains(KeyModifiers::SHIFT)) {
            app.undo();
            return;
        }
    }
    if ctrl_shift(&key, 'z') {
        app.redo();
        return;
    }
    if ctrl_shift(&key, 'c') {
        app.copy_to_system();
        return;
    }
    if ctrl_shift(&key, 'v') {
        app.paste_from_system();
        return;
    }
    match key.code {
        KeyCode::Tab => {
            app.insert_tab();
            return;
        }
        KeyCode::BackTab => {
            app.dedent_current_line();
            return;
        }
        _ => {}
    }

    // Fall through to textarea (arrows, typing, undo via textarea defaults excluded above)
    // Avoid double-handling Ctrl+Z when shift present already handled; let textarea handle the rest.
    // Strip our already-handled Ctrl combos: don't pass Ctrl+S/N/Q/F/H to textarea.
    if key.modifiers.contains(KeyModifiers::CONTROL)
        && matches!(key.code, KeyCode::Char(c) if "snqfhzcv".contains(c.to_ascii_lowercase()))
    {
        return;
    }
    let input: Input = key.into();
    // ratatui-textarea Input from KeyEvent
    if app.textarea.input(input) {
        app.mark_edited();
    }
}

fn delete_word_backward_str(s: &mut String) {
    if s.is_empty() {
        return;
    }
    // Remove one trailing-space run, else one word run.
    if s.ends_with(char::is_whitespace) {
        while s.ends_with(char::is_whitespace) {
            s.pop();
        }
    } else {
        while let Some(c) = s.chars().last() {
            if c.is_whitespace() {
                break;
            }
            s.pop();
        }
    }
}

fn handle_save_prompt_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Editing;
            app.save_input.clear();
            app.message = "Save cancelled".to_string();
        }
        KeyCode::Enter => app.confirm_save(),
        KeyCode::Backspace => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                delete_word_backward_str(&mut app.save_input);
            } else {
                app.save_input.pop();
            }
        }
        KeyCode::Char(c) => {
            if !key.modifiers.contains(KeyModifiers::CONTROL)
                && !key.modifiers.contains(KeyModifiers::ALT)
            {
                app.save_input.push(c);
            }
        }
        _ => {}
    }
}

fn handle_overwrite_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char(c) if c.to_ascii_lowercase() == 'y' => app.confirm_overwrite(true),
        KeyCode::Char(c) if c.to_ascii_lowercase() == 'n' => app.confirm_overwrite(false),
        KeyCode::Enter => app.confirm_overwrite(true),
        KeyCode::Esc => app.confirm_overwrite(false),
        _ => {}
    }
}

fn handle_find_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.close_find();
        }
        KeyCode::Enter => app.apply_find(true),
        KeyCode::Backspace => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                delete_word_backward_str(&mut app.find_input);
            } else {
                app.find_input.pop();
            }
            app.apply_find(false);
        }
        KeyCode::Char(c) => {
            if !key.modifiers.contains(KeyModifiers::CONTROL)
                && !key.modifiers.contains(KeyModifiers::ALT)
            {
                app.find_input.push(c);
                app.apply_find(false);
            } else if ctrl(&key, 'q') {
                app.should_quit = true;
            }
        }
        _ => {}
    }
}

fn handle_replace_key(app: &mut App, key: KeyEvent) {
    // Ctrl+A = replace all, Tab = switch field, Enter = replace current
    if key.modifiers == KeyModifiers::CONTROL && matches!(key.code, KeyCode::Char(c) if c.to_ascii_lowercase() == 'a')
    {
        app.apply_replace_all();
        return;
    }
    match key.code {
        KeyCode::Esc => {
            app.close_replace();
        }
        KeyCode::Tab | KeyCode::BackTab => {
            app.replace_focus_find = !app.replace_focus_find;
        }
        KeyCode::Enter => app.apply_replace_current(),
        KeyCode::Backspace => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                if app.replace_focus_find {
                    delete_word_backward_str(&mut app.find_input);
                } else {
                    delete_word_backward_str(&mut app.replace_input);
                }
            } else if app.replace_focus_find {
                app.find_input.pop();
            } else {
                app.replace_input.pop();
            }
        }
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL)
                || key.modifiers.contains(KeyModifiers::ALT)
            {
                return;
            }
            if app.replace_focus_find {
                app.find_input.push(c);
            } else {
                app.replace_input.push(c);
            }
        }
        _ => {}
    }
}

// ---- Mouse: click to move, drag to select (same pattern as ratatui-textarea minimal example) ----

fn clicked_cursor(area: Rect, app: &App, ev: &MouseEvent) -> Option<CursorMove> {
    let inner = app
        .textarea
        .block()
        .map_or(area, |b| b.inner(area));
    if !inner.contains(Position::new(ev.column, ev.row)) {
        return None;
    }
    let (top_row, top_col) = app.textarea.scroll_offset();
    let row = ev.row.saturating_sub(inner.y) + top_row;
    let col = ev
        .column
        .saturating_sub(inner.x)
        .saturating_sub(app.textarea.line_number_width())
        + top_col;
    let DataCursor(r, c) = app
        .textarea
        .screen_to_data(usize::from(row), usize::from(col));
    Some(CursorMove::Jump(r as u16, c as u16))
}

fn handle_mouse(app: &mut App, area: &Rect, ev: MouseEvent) {
    if !matches!(app.mode, Mode::Editing) {
        return;
    }
    match ev.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if let Some(cursor) = clicked_cursor(*area, app, &ev) {
                app.textarea.cancel_selection();
                app.textarea.move_cursor(cursor);
                app.textarea.start_selection();
            }
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            if let Some(cursor) = clicked_cursor(*area, app, &ev) {
                app.textarea.move_cursor(cursor);
            }
        }
        MouseEventKind::Up(MouseButton::Left) => {
            if app
                .textarea
                .selection_range()
                .is_some_and(|(from, to)| from == to)
            {
                app.textarea.cancel_selection();
            }
        }
        _ => {
            // Wheel scroll etc.
            let input: Input = ev.into();
            app.textarea.input(input);
        }
    }
}
