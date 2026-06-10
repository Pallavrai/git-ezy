use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::App;
use crate::types;

impl App {
    pub(super) fn render_commit_list(&self, frame: &mut Frame, area: Rect) {
        let title = format!(" History ({}) ", self.commits.len());
        let lines: Vec<Line> = self
            .commits
            .iter()
            .enumerate()
            .map(|(i, commit)| {
                let selected = i == self.selected_commit;
                let bg = if selected { Color::Cyan } else { Color::Reset };
                let fg = if selected { Color::Black } else { Color::White };

                let tree = self
                    .tree_lines
                    .get(i)
                    .map(|s| s.as_str())
                    .unwrap_or("");

                let hash_color = if selected { Color::Black } else { Color::Yellow };
                let ref_str = if !commit.refs.is_empty() {
                    format!(" ({})", commit.refs.join(", "))
                } else {
                    String::new()
                };
                let time_str = types::format_relative_time(commit.time);

                let subject = if commit.subject.len() > 50 {
                    format!(" {}...", &commit.subject[..47])
                } else {
                    format!(" {}", commit.subject)
                };

                let spans = vec![
                    Span::styled(
                        tree,
                        Style::default().fg(if selected { Color::Black } else { Color::Cyan }).bg(bg),
                    ),
                    Span::styled(
                        format!(" {} ", commit.hash),
                        Style::default().fg(hash_color).bg(bg),
                    ),
                    Span::styled(
                        ref_str,
                        Style::default()
                            .fg(if selected { Color::Black } else { Color::Green })
                            .add_modifier(Modifier::BOLD)
                            .bg(bg),
                    ),
                    Span::styled(
                        format!(" {}", time_str),
                        Style::default()
                            .fg(if selected { Color::Black } else { Color::DarkGray })
                            .bg(bg),
                    ),
                    Span::styled(
                        subject,
                        Style::default().fg(fg).bg(bg),
                    ),
                ];

                Line::from(spans)
            })
            .collect();

        let text = Text::from(lines);
        let view_h = area.height.saturating_sub(2) as usize;

        let vert = self
            .list_scroll_offset
            .min(text.height().saturating_sub(view_h));
        let para = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(self.pane_border_style(false))
                    .title(title.as_str())
                    .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            )
            .scroll((vert as u16, self.h_scroll_offset as u16));
        frame.render_widget(para, area);
    }
}
