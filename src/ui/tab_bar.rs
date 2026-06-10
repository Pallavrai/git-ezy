use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::app::App;
use crate::types::MENU_ITEMS;

impl App {
    pub(super) fn render_tab_bar(&self, frame: &mut Frame, area: Rect) {
        let widths = [12, 12, 12, 14];
        let tabs_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                widths
                    .iter()
                    .map(|w| Constraint::Length(*w))
                    .collect::<Vec<_>>(),
            )
            .split(area);

        let block = Block::default()
            .borders(Borders::BOTTOM)
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(Color::DarkGray));

        for (i, name) in MENU_ITEMS.iter().enumerate() {
            let is_active = i == self.current_tab;
            let tab_style = if is_active {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            let prefix = if is_active {
                Span::styled(" ● ", Style::default().fg(Color::Cyan))
            } else {
                Span::styled("   ", Style::default())
            };
            let label = Span::styled(*name, tab_style);
            let line = Line::from(vec![prefix, label]);
            let para = Paragraph::new(line);
            frame.render_widget(para, tabs_layout[i]);
        }

        frame.render_widget(block, area);
    }
}
