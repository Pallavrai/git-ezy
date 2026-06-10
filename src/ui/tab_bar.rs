use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::App;
use crate::types::MENU_ITEMS;

impl App {
    pub(super) fn render_tab_bar(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::DarkGray));

        frame.render_widget(block, area);

        let gap = 1u16;
        let constraints: Vec<Constraint> = MENU_ITEMS
            .iter()
            .map(|name| Constraint::Length(name.len() as u16 + 4))
            .collect();

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .horizontal_margin(2)
            .spacing(gap)
            .constraints(constraints)
            .split(area);

        for (i, name) in MENU_ITEMS.iter().enumerate() {
            let is_active = i == self.current_tab;
            let style = if is_active {
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            let total = name.len() + 4;
            let text = format!("{:^1$}", name, total);
            let span = Span::styled(text, style);
            frame.render_widget(Paragraph::new(Line::from(span)), chunks[i]);
        }
    }
}
