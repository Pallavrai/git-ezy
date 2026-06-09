use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use crate::app::App;

impl App {
    pub(super) fn render_branch_list(&self, frame: &mut Frame, area: Rect) {
        let title = format!(" Branches ({}) ", self.branches.len());
        let items: Vec<ListItem> = self
            .branches
            .iter()
            .enumerate()
            .map(|(i, branch)| {
                let selected = i == self.selected_branch;
                let bg = if selected { Color::Cyan } else { Color::Reset };
                let fg = if selected {
                    Color::Black
                } else if branch.is_current {
                    Color::Green
                } else {
                    Color::White
                };

                let marker = if branch.is_current {
                    " *"
                } else {
                    "  "
                };
                let name_style = Style::default()
                    .fg(fg)
                    .bg(bg)
                    .add_modifier(
                        if branch.is_current {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        },
                    );
                ListItem::new(Line::from(vec![Span::styled(
                    format!("{marker} {}", branch.name),
                    name_style,
                )]))
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
