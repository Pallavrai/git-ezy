use anyhow::Result;
use git2::{DiffOptions, Repository, Status};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub const MENU_ITEMS: &[&str] = &["Status", "Stage Files", "Commit", "Branches"];

#[derive(Debug, Clone)]
pub struct FileItem {
    pub path: String,
    pub status: Status,
}

pub struct App {
    pub current_tab: usize,
    pub selected_index: usize,
    pub files: Vec<FileItem>,
    pub repo: Option<Repository>,
    pub should_quit: bool,
    pub diff_text: String,
    pub branch: String,
}

impl App {
    pub fn new() -> Self {
        let repo = Repository::open(".").ok();
        let branch = repo
            .as_ref()
            .and_then(|r| r.head().ok())
            .and_then(|h| h.shorthand().map(String::from))
            .unwrap_or_else(|| "detached".into());

        let mut app = Self {
            current_tab: 0,
            selected_index: 0,
            files: Vec::new(),
            repo,
            should_quit: false,
            diff_text: String::new(),
            branch,
        };
        app.refresh_status();
        app
    }

    pub fn refresh_status(&mut self) {
        self.files.clear();
        let repo = match &self.repo {
            Some(r) => r,
            None => return,
        };
        let statuses = match repo.statuses(None) {
            Ok(s) => s,
            Err(_) => return,
        };
        for entry in statuses.iter() {
            let path = match entry.path() {
                Some(p) => p.to_string(),
                None => continue,
            };
            let status = entry.status();
            if status != Status::CURRENT {
                self.files.push(FileItem { path, status });
            }
        }
        self.selected_index = self
            .selected_index
            .min(self.files.len().saturating_sub(1));
    }

    pub fn toggle_stage(&mut self) -> Result<()> {
        let repo = match &self.repo {
            Some(r) => r,
            None => return Ok(()),
        };
        if self.files.is_empty() {
            return Ok(());
        }
        let item = &self.files[self.selected_index];
        let path = std::path::Path::new(&item.path);

        let is_staged = item.status.intersects(
            Status::INDEX_NEW
                | Status::INDEX_MODIFIED
                | Status::INDEX_DELETED
                | Status::INDEX_RENAMED
                | Status::INDEX_TYPECHANGE,
        );

        let mut index = repo.index()?;
        if is_staged {
            index.remove_path(path)?;
        } else {
            index.add_path(path)?;
        }
        index.write()?;
        self.refresh_status();
        self.update_diff()?;
        Ok(())
    }

    pub fn update_diff(&mut self) -> Result<()> {
        self.diff_text.clear();
        let repo = match &self.repo {
            Some(r) => r,
            None => return Ok(()),
        };
        if self.files.is_empty() {
            return Ok(());
        }
        let item = &self.files[self.selected_index];
        let path = std::path::Path::new(&item.path);

        let mut opts = DiffOptions::new();
        opts.pathspec(path.to_str().unwrap_or(""));

        let tree = match repo.head() {
            Ok(h) => h.peel_to_tree().ok(),
            Err(_) => None,
        };

        let diff = repo.diff_tree_to_workdir(tree.as_ref(), Some(&mut opts))?;
        let mut text = String::new();
        diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
            let prefix = match line.origin() {
                '+' => "+",
                '-' => "-",
                _ => " ",
            };
            if let Ok(content) = std::str::from_utf8(line.content()) {
                text.push_str(&format!("{}{}", prefix, content));
            }
            true
        })?;
        self.diff_text = text;
        Ok(())
    }

    pub fn next(&mut self) {
        if self.files.is_empty() {
            return;
        }
        self.selected_index = (self.selected_index + 1) % self.files.len();
        let _ = self.update_diff();
    }

    pub fn previous(&mut self) {
        if self.files.is_empty() {
            return;
        }
        self.selected_index = if self.selected_index == 0 {
            self.files.len() - 1
        } else {
            self.selected_index - 1
        };
        let _ = self.update_diff();
    }

    pub fn next_tab(&mut self) {
        self.current_tab = (self.current_tab + 1) % MENU_ITEMS.len();
        self.selected_index = 0;
        if self.current_tab == 0 || self.current_tab == 1 {
            let _ = self.update_diff();
        }
    }

    pub fn previous_tab(&mut self) {
        self.current_tab = if self.current_tab == 0 {
            MENU_ITEMS.len() - 1
        } else {
            self.current_tab - 1
        };
        self.selected_index = 0;
        if self.current_tab == 0 || self.current_tab == 1 {
            let _ = self.update_diff();
        }
    }

    pub fn active_menu_name(&self) -> &str {
        MENU_ITEMS[self.current_tab]
    }

    fn status_style(status: Status) -> (char, Color) {
        if status == Status::WT_NEW {
            ('?', Color::Yellow)
        } else if status == Status::WT_MODIFIED {
            ('M', Color::Yellow)
        } else if status == Status::WT_DELETED {
            ('D', Color::Red)
        } else if status.intersects(Status::WT_RENAMED) {
            ('R', Color::Yellow)
        } else if status.intersects(Status::WT_TYPECHANGE) {
            ('T', Color::Yellow)
        } else if status.intersects(Status::INDEX_NEW) {
            ('A', Color::Green)
        } else if status.intersects(Status::INDEX_MODIFIED) {
            ('M', Color::Green)
        } else if status.intersects(Status::INDEX_DELETED) {
            ('D', Color::Red)
        } else if status.intersects(Status::INDEX_RENAMED) {
            ('R', Color::Green)
        } else if status.intersects(Status::INDEX_TYPECHANGE) {
            ('T', Color::Green)
        } else {
            ('\u{00B7}', Color::DarkGray)
        }
    }

    fn help_text(&self) -> &'static str {
        match self.current_tab {
            0 => "↑↓ Navigate  Enter:View Diff  Tab:Switch  q:Quit",
            1 => "↑↓ Navigate  Space:Stage/Unstage  Tab:Switch  q:Quit",
            _ => "Tab:Switch  q:Quit",
        }
    }

    pub fn render(&self, frame: &mut Frame) {
        let main = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(frame.area());

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
            .split(main[1]);

        self.render_tab_bar(frame, main[0]);
        self.render_left_pane(frame, chunks[0]);
        self.render_right_pane(frame, chunks[1]);
        self.render_status_bar(frame, main[2]);
    }

    fn render_tab_bar(&self, frame: &mut Frame, area: Rect) {
        let widths = [15, 15, 12, 14];
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

    fn render_left_pane(&self, frame: &mut Frame, area: Rect) {
        match self.current_tab {
            0 | 1 => self.render_file_list(frame, area),
            _ => self.render_placeholder_list(frame, area),
        }
    }

    fn render_file_list(&self, frame: &mut Frame, area: Rect) {
        let title = format!(
            " {} ({}) ",
            self.active_menu_name(),
            self.files.len()
        );
        let items: Vec<ListItem> = self
            .files
            .iter()
            .enumerate()
            .map(|(i, file)| {
                let selected = i == self.selected_index;
                let (status_char, status_color) = Self::status_style(file.status);

                let staged_marker = if self.current_tab == 1
                    && file.status.intersects(
                        Status::INDEX_NEW
                            | Status::INDEX_MODIFIED
                            | Status::INDEX_DELETED,
                    )
                {
                    Span::styled(" ✓", Style::default().fg(Color::Green))
                } else if self.current_tab == 1 {
                    Span::styled("  ", Style::default())
                } else {
                    Span::styled("", Style::default())
                };

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

        let border_style = Style::default().fg(Color::DarkGray);
        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(title.as_str())
                .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        );
        frame.render_widget(list, area);
    }

    fn render_placeholder_list(&self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = (0..MENU_ITEMS.len())
            .map(|i| {
                let selected = i == self.current_tab;
                let style = if selected {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Gray)
                };
                let prefix = if selected { "▶ " } else { "  " };
                ListItem::new(Line::from(Span::styled(
                    format!("{}{}", prefix, MENU_ITEMS[i]),
                    style,
                )))
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(format!(" {} ", self.active_menu_name()))
                .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        );
        frame.render_widget(list, area);
    }

    fn render_right_pane(&self, frame: &mut Frame, area: Rect) {
        let title = match self.current_tab {
            0 => " Diff ",
            1 => " Stage Details ",
            2 => " Commit ",
            3 => " Branches ",
            _ => "",
        };

        let border_style = Style::default().fg(Color::DarkGray);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
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
                        "  No files to stage.",
                        Style::default().fg(Color::Gray),
                    )))
                } else {
                    let file = &self.files[self.selected_index];
                    let (status_char, status_color) = Self::status_style(file.status);
                    Text::from(vec![
                        Line::from(Span::styled(
                            "  Selected file:",
                            Style::default().fg(Color::Gray),
                        )),
                        Line::from(""),
                        Line::from(vec![
                            Span::styled(
                                format!("  [{}]", status_char),
                                Style::default()
                                    .fg(Color::Black)
                                    .bg(status_color)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(
                                format!(" {}", file.path),
                                Style::default().fg(Color::White),
                            ),
                        ]),
                        Line::from(""),
                        Line::from(Span::styled(
                            "  Press SPACE to toggle staging",
                            Style::default().fg(Color::Cyan),
                        )),
                    ])
                }
            }
            2 => Text::from(vec![
                Line::from(Span::styled(
                    "  Commit view",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  Coming soon...",
                    Style::default().fg(Color::Gray),
                )),
            ]),
            3 => Text::from(vec![
                Line::from(Span::styled(
                    "  Branches",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  Current branch:",
                    Style::default().fg(Color::Gray),
                )),
                Line::from(Span::styled(
                    format!("  {}", self.branch),
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                )),
            ]),
            _ => Text::from(Line::from(Span::raw(""))),
        };

        let para = Paragraph::new(text)
            .block(block)
            .wrap(Wrap { trim: false });
        frame.render_widget(para, area);
    }

    fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
        let branch_span = Span::styled(
            format!(" ◇ {} ", self.branch),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

        let help = self.help_text();
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
            let help_len = help.len();
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
                    .saturating_sub(self.branch.len() + 3 + 5 + help.len()),
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
