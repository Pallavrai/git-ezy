use anyhow::Result;
use git2::{DiffOptions, Repository, Status};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
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
}

impl App {
    pub fn new() -> Self {
        let repo = Repository::open(".").ok();
        let mut app = Self {
            current_tab: 0,
            selected_index: 0,
            files: Vec::new(),
            repo,
            should_quit: false,
            diff_text: String::new(),
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

    pub fn render(&self, frame: &mut Frame) {
        let main = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(frame.area());

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
            .split(main[1]);

        self.render_tab_bar(frame, main[0]);
        self.render_left_pane(frame, chunks[0]);
        self.render_right_pane(frame, chunks[1]);
    }

    fn render_tab_bar(&self, frame: &mut Frame, area: Rect) {
        let spans: Vec<Span> = MENU_ITEMS
            .iter()
            .enumerate()
            .flat_map(|(i, name)| {
                let style = if i == self.current_tab {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White).bg(Color::DarkGray)
                };
                let sep = if i > 0 {
                    vec![Span::styled(" │ ", Style::default().fg(Color::Gray))]
                } else {
                    vec![]
                };
                let mut v = sep;
                v.push(Span::styled(*name, style));
                v
            })
            .collect();

        let block = Block::default().borders(Borders::ALL).title(" git-ezy ");
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let line = Line::from(spans);
        let para = Paragraph::new(line);
        frame.render_widget(para, inner);
    }

    fn render_left_pane(&self, frame: &mut Frame, area: Rect) {
        match self.current_tab {
            0 | 1 => self.render_file_list(frame, area),
            _ => self.render_placeholder_list(frame, area),
        }
    }

    fn render_file_list(&self, frame: &mut Frame, area: Rect) {
        let title = format!("{} ({})", self.active_menu_name(), self.files.len());
        let items: Vec<ListItem> = self
            .files
            .iter()
            .enumerate()
            .map(|(i, file)| {
                let selected = i == self.selected_index;

                let staged_prefix = if self.current_tab == 1 {
                    if file.status.intersects(
                        Status::INDEX_NEW
                            | Status::INDEX_MODIFIED
                            | Status::INDEX_DELETED,
                    ) {
                        "✓ "
                    } else {
                        "  "
                    }
                } else {
                    ""
                };

                let status_char = if file.status == Status::WT_NEW {
                    'N'
                } else if file.status == Status::WT_MODIFIED {
                    'M'
                } else if file.status == Status::WT_DELETED {
                    'D'
                } else if file.status.intersects(Status::INDEX_NEW) {
                    'A'
                } else if file.status.intersects(Status::INDEX_MODIFIED) {
                    'M'
                } else if file.status.intersects(Status::INDEX_DELETED) {
                    'D'
                } else {
                    '?'
                };

                let label = format!("{} [{}] {}", staged_prefix, status_char, file.path);
                let style = if selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                ListItem::new(Line::from(Span::styled(label, style)))
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(title.as_str()),
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
                    Style::default().fg(Color::White)
                };
                ListItem::new(Line::from(Span::styled(MENU_ITEMS[i], style)))
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(self.active_menu_name()),
        );
        frame.render_widget(list, area);
    }

    fn render_right_pane(&self, frame: &mut Frame, area: Rect) {
        let title = match self.current_tab {
            0 => "Diff (workdir vs HEAD)",
            1 => "Details",
            2 => "Commit Message",
            3 => "Branches",
            _ => "",
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .style(Style::default().fg(Color::White));

        let text = match self.current_tab {
            0 => self.diff_text.clone(),
            1 => {
                if self.files.is_empty() {
                    "No files to stage.".into()
                } else {
                    format!(
                        "Press SPACE to toggle staging for:\n{}",
                        self.files
                            .get(self.selected_index)
                            .map(|f| f.path.as_str())
                            .unwrap_or("")
                    )
                }
            }
            2 => "Commit view (coming soon)".into(),
            3 => "Branches view (coming soon)".into(),
            _ => String::new(),
        };

        if text.is_empty() {
            let para = Paragraph::new(Text::from(
                if self.files.is_empty() {
                    "No changes in working tree."
                } else {
                    "Select a file to view diff."
                },
            ))
            .block(block)
            .wrap(Wrap { trim: false });
            frame.render_widget(para, area);
        } else {
            let para = Paragraph::new(Text::from(text.as_str()))
                .block(block)
                .wrap(Wrap { trim: false });
            frame.render_widget(para, area);
        }
    }
}
