use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::types;

impl App {
    pub(super) fn render_right_pane(&self, frame: &mut Frame, area: Rect) {
        let title = match self.current_tab {
            0 => " Diff ",
            1 => " Stage Details ",
            2 => " History ",
            3 => " Branches ",
            _ => "",
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.pane_border_style(true))
            .title(title)
            .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

        let text = match self.current_tab {
            0 => {
                if self.diff_text.is_empty() {
                    Text::from(Line::from(Span::styled(
                        if self.files.is_empty() {
                            "  No changes in working tree."
                        } else {
                            "  Select a file to view diff."
                        },
                        Style::default().fg(Color::Gray),
                    )))
                } else {
                    let lines: Vec<Line> = self
                        .diff_text
                        .lines()
                        .map(|l| {
                            let (fg, bg) = if l.starts_with('+') {
                                (Color::Green, Color::Reset)
                            } else if l.starts_with('-') {
                                (Color::Red, Color::Reset)
                            } else if l.starts_with("@@") {
                                (Color::Cyan, Color::Reset)
                            } else {
                                (Color::White, Color::Reset)
                            };
                            let modifier = if l.starts_with("@@") {
                                Modifier::BOLD
                            } else {
                                Modifier::empty()
                            };
                            Line::from(Span::styled(
                                l,
                                Style::default().fg(fg).bg(bg).add_modifier(modifier),
                            ))
                        })
                        .collect();
                    Text::from(lines)
                }
            }
            1 => {
                if self.files.is_empty() {
                    Text::from(Line::from(Span::styled(
                        "  No changes.",
                        Style::default().fg(Color::Gray),
                    )))
                } else {
                    Text::from(vec![
                        Line::from(""),
                        Line::from(Span::styled(
                            "  a  Stage All",
                            Style::default().fg(Color::Green),
                        )),
                        Line::from(Span::styled(
                            "  u  Unstage All",
                            Style::default().fg(Color::Yellow),
                        )),
                        Line::from(Span::styled(
                            "  d  Discard All",
                            Style::default().fg(Color::Red),
                        )),
                    ])
                }
            }
            2 => {
                if self.commits.is_empty() {
                    Text::from(Line::from(Span::styled(
                        "  No commits found.",
                        Style::default().fg(Color::Gray),
                    )))
                } else if self.focus == types::Focus::List {
                    Text::from(Line::from(Span::styled(
                        "  Select a commit and press Enter to view details.",
                        Style::default().fg(Color::Gray),
                    )))
                } else {
                    let idx = self
                        .selected_commit
                        .min(self.commits.len().saturating_sub(1));
                    let commit = &self.commits[idx];
                    let refs_str = if !commit.refs.is_empty() {
                        format!(" ({})", commit.refs.join(", "))
                    } else {
                        String::new()
                    };
                    let time_str = types::format_relative_time(commit.time);

                    let header = vec![
                        Line::from(Span::styled(
                            " Commit Details",
                            Style::default().fg(Color::Gray),
                        )),
                        Line::from(""),
                        Line::from(vec![
                            Span::styled(" Hash:   ", Style::default().fg(Color::Gray)),
                            Span::styled(
                                commit.hash.as_str(),
                                Style::default().fg(Color::Yellow),
                            ),
                        ]),
                        Line::from(vec![
                            Span::styled(" Author: ", Style::default().fg(Color::Gray)),
                            Span::styled(
                                commit.author.as_str(),
                                Style::default().fg(Color::White),
                            ),
                        ]),
                        Line::from(vec![
                            Span::styled(" Date:   ", Style::default().fg(Color::Gray)),
                            Span::styled(
                                format!("{} ago", time_str),
                                Style::default().fg(Color::White),
                            ),
                        ]),
                        Line::from(vec![
                            Span::styled(" Ref:    ", Style::default().fg(Color::Gray)),
                            Span::styled(
                                refs_str,
                                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                            ),
                        ]),
                        Line::from(""),
                        Line::from(Span::styled(
                            format!("    {}", commit.subject),
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        )),
                    ];

                    let body_lines: Vec<Line> = commit
                        .message
                        .lines()
                        .skip(1)
                        .map(|l| {
                            Line::from(Span::styled(
                                format!("    {}", l),
                                Style::default().fg(Color::White),
                            ))
                        })
                        .collect();

                    let mut all = header;
                    all.extend(body_lines);
                    Text::from(all)
                }
            }
            3 => {
                if self.branches.is_empty() {
                    Text::from(Line::from(Span::styled(
                        "  No branches found.",
                        Style::default().fg(Color::Gray),
                    )))
                } else {
                    let idx = self
                        .selected_branch
                        .min(self.branches.len().saturating_sub(1));
                    let branch = &self.branches[idx];
                    let current_tag = if branch.is_current {
                        " (current)"
                    } else {
                        ""
                    };
                    Text::from(vec![
                        Line::from(Span::styled(
                            format!(" ◇ {}{}", branch.name, current_tag),
                            Style::default()
                                .fg(if branch.is_current {
                                    Color::Green
                                } else {
                                    Color::Cyan
                                })
                                .add_modifier(Modifier::BOLD),
                        )),
                        Line::from(""),
                        Line::from(vec![
                            Span::styled(" Hash:    ", Style::default().fg(Color::Gray)),
                            Span::styled(
                                branch.oid.to_string(),
                                Style::default().fg(Color::Yellow),
                            ),
                        ]),
                        Line::from(vec![
                            Span::styled(" Upstream:", Style::default().fg(Color::Gray)),
                            Span::styled(
                                format!(
                                    " {}",
                                    branch.upstream.as_deref().unwrap_or("(none)")
                                ),
                                Style::default().fg(Color::White),
                            ),
                        ]),
                        Line::from(""),
                        Line::from(Span::styled(
                            if branch.is_current {
                                "  This is the current branch."
                            } else {
                                "  Press 'c' to checkout this branch."
                            },
                            Style::default().fg(Color::Gray),
                        )),
                    ])
                }
            }
            _ => Text::from(Line::from(Span::raw(""))),
        };

        let content_h = text.height();
        let view_h = area.height.saturating_sub(2) as usize;
        let scroll = self
            .scroll_offset
            .min(content_h.saturating_sub(view_h));
        let para = Paragraph::new(text)
            .block(block)
            .scroll((scroll as u16, 0))
            .wrap(Wrap { trim: false });
        frame.render_widget(para, area);
    }
}
