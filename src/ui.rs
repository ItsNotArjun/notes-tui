use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::app::{App, Mode};

pub fn draw(f: &mut Frame, app: &mut App, editor_area: &mut Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(f.area());

    *editor_area = chunks[0];

    let title = match &app.last_saved_export {
        Some(p) => format!(" notes-tui — {} ", p.display()),
        None => " notes-tui ".to_string(),
    };
    app.textarea.set_block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .title_style(Style::default().fg(Color::Green))
            .border_style(Style::default().fg(Color::Green)),
    );
    f.render_widget(&app.textarea, chunks[0]);

    let status = Paragraph::new(Line::from(vec![Span::raw(app.message.clone())]))
        .style(Style::default().fg(Color::Gray).bg(Color::Black))
        .alignment(Alignment::Left);
    f.render_widget(status, chunks[1]);

    match &app.mode {
        Mode::SavePrompt => {
            let popup = centered_popup(f.area(), 60, 5);
            f.render_widget(Clear, popup);
            let text = format!("Save to ~/Downloads/notes/\n> {}", app.save_input);
            let p = Paragraph::new(text).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Save As (.txt auto-added) — Enter save, Esc cancel")
                    .title_style(Style::default().fg(Color::Green))
                    .border_style(Style::default().fg(Color::Green)),
            );
            f.render_widget(p, popup);
        }
        Mode::OverwriteConfirm(path) => {
            let popup = centered_popup(f.area(), 60, 5);
            f.render_widget(Clear, popup);
            let text = format!("Exists:\n{}\nOverwrite? (y/n)", path.display());
            let p = Paragraph::new(text).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Confirm overwrite")
                    .title_style(Style::default().fg(Color::Green))
                    .border_style(Style::default().fg(Color::Green)),
            );
            f.render_widget(p, popup);
        }
        Mode::FindPrompt => {
            let popup = centered_popup(f.area(), 60, 4);
            f.render_widget(Clear, popup);
            let text = format!("Find: {}\nEnter next, Esc close", app.find_input);
            let p = Paragraph::new(text).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Find — Ctrl+F")
                    .title_style(Style::default().fg(Color::Green))
                    .border_style(Style::default().fg(Color::Green)),
            );
            f.render_widget(p, popup);
        }
        Mode::ReplacePrompt => {
            let popup = centered_popup(f.area(), 60, 7);
            f.render_widget(Clear, popup);
            let marker_find = if app.replace_focus_find { ">" } else { " " };
            let marker_rep = if app.replace_focus_find { " " } else { ">" };
            let text = format!(
                "{} Find: {}\n{} Replace: {}\nEnter replace-one, Ctrl+A all, Tab switch, Esc close",
                marker_find, app.find_input, marker_rep, app.replace_input
            );
            let p = Paragraph::new(text).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Replace — Ctrl+H")
                    .title_style(Style::default().fg(Color::Green))
                    .border_style(Style::default().fg(Color::Green)),
            );
            f.render_widget(p, popup);
        }
        Mode::Editing => {}
    }
}

fn centered_popup(area: Rect, width_pct: u16, height: u16) -> Rect {
    let width = (area.width * width_pct / 100).max(30).min(area.width);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect {
        x,
        y,
        width,
        height: height.min(area.height),
    }
}
