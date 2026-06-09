use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::App;

impl App {
    pub(super) fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
        let branch_span = Span::styled(
            format!(" ◇ {} ", self.branch),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

        let help = self.help_text();
        let help_len = help.len();
        let help_span = Span::styled(
            help,
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

        let (left, right) = {
            let area_width = area.width as usize;
            let branch_len = self.branch.len() + 3;
            let sep = "  │  ";
            let total = branch_len + sep.len() + help_len;
            if total <= area_width {
                (true, true)
            } else if branch_len + 3 <= area_width {
                (true, false)
            } else {
                (false, false)
            }
        };

        let spans = if left && right {
            let padding = " ".repeat(
                (area.width as usize)
                    .saturating_sub(self.branch.len() + 3 + 5 + help_len),
            );
            vec![
                branch_span,
                Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
                Span::styled(padding, Style::default()),
                help_span,
            ]
        } else if left {
            vec![branch_span]
        } else {
            vec![]
        };

        let line = Line::from(spans);
        let para = Paragraph::new(line).style(
            Style::default()
                .bg(Color::Rgb(20, 20, 30))
                .fg(Color::White),
        );
        frame.render_widget(para, area);
    }
}
