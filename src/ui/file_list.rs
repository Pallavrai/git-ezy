use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use crate::app::App;

impl App {
    pub(super) fn render_file_list(&self, frame: &mut Frame, area: Rect) {
        let title = format!(" {} ({}) ", self.active_menu_name(), self.files.len());
        let items: Vec<ListItem> = self
            .files
            .iter()
            .enumerate()
            .map(|(i, file)| {
                let selected = i == self.selected_index;
                let (status_char, status_color) = Self::status_style(file.status);

                let staged_marker = Span::styled("", Style::default());

                let status_tag = Span::styled(
                    format!(" {} ", status_char),
                    Style::default()
                        .fg(Color::Black)
                        .bg(status_color)
                        .add_modifier(Modifier::BOLD),
                );

                let path_style = if selected {
                    Style::default().fg(Color::Black).bg(Color::Cyan)
                } else {
                    Style::default().fg(Color::White)
                };
                let path_span = Span::styled(format!(" {}", file.path), path_style);

                let spans = vec![staged_marker, status_tag, path_span];
                ListItem::new(Line::from(spans))
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(self.pane_border_style(false))
                .title(title.as_str())
                .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        );
        frame.render_widget(list, area);
    }
}
