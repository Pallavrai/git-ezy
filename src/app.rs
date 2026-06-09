use std::collections::HashMap;

use anyhow::Result;
use git2::{DiffOptions, Repository, Status, Oid};
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

#[derive(Debug, Clone)]
pub struct CommitEntry {
    pub oid: Oid,
    pub hash: String,
    pub author: String,
    pub time: i64,
    pub subject: String,
    pub message: String,
    pub refs: Vec<String>,
    pub parents: Vec<Oid>,
}

pub const SCROLL_STEP: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    List,
    Detail,
}

pub struct App {
    pub current_tab: usize,
    pub selected_index: usize,
    pub selected_commit: usize,
    pub files: Vec<FileItem>,
    pub commits: Vec<CommitEntry>,
    pub tree_lines: Vec<String>,
    pub repo: Option<Repository>,
    pub should_quit: bool,
    pub diff_text: String,
    pub commit_detail: String,
    pub branch: String,
    pub scroll_offset: usize,
    pub focus: Focus,
}

fn format_relative_time(seconds: i64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let diff = (now - seconds).max(0);
    if diff < 60 {
        format!("{diff}s")
    } else if diff < 3600 {
        format!("{}m", diff / 60)
    } else if diff < 86400 {
        format!("{}h", diff / 3600)
    } else if diff < 2592000 {
        format!("{}d", diff / 86400)
    } else {
        format!("{}mo", diff / 2592000)
    }
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
            selected_commit: 0,
            files: Vec::new(),
            commits: Vec::new(),
            tree_lines: Vec::new(),
            repo,
            should_quit: false,
            diff_text: String::new(),
            commit_detail: String::new(),
            branch,
            scroll_offset: 0,
            focus: Focus::List,
        };
        app.refresh_status();
        app.load_commits();
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

    pub fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(SCROLL_STEP);
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(SCROLL_STEP);
    }

    pub fn update_diff(&mut self) -> Result<()> {
        self.diff_text.clear();
        self.scroll_offset = 0;
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

    fn load_commits(&mut self) {
        self.commits.clear();
        let repo = match &self.repo {
            Some(r) => r,
            None => return,
        };

        let mut ref_map: HashMap<Oid, Vec<String>> = HashMap::new();
        if let Ok(refs) = repo.references() {
            for reference in refs.flatten() {
                if let Some(name) = reference.shorthand()
                    && (reference.is_tag() || reference.is_branch())
                    && let Some(target) = reference.target()
                {
                    ref_map.entry(target).or_default().push(name.to_string());
                }
            }
        }

        let mut revwalk = match repo.revwalk() {
            Ok(w) => w,
            Err(_) => return,
        };
        let _ = revwalk.push_glob("*");
        revwalk.set_sorting(git2::Sort::TIME | git2::Sort::TOPOLOGICAL).ok();

        for oid in revwalk.flatten() {
            if let Ok(commit) = repo.find_commit(oid) {
                let labels = ref_map.remove(&oid).unwrap_or_default();
                let author = commit.author().name().unwrap_or("unknown").to_string();
                let subject = commit
                    .message()
                    .and_then(|m| m.lines().next())
                    .unwrap_or("")
                    .to_string();
                let body = commit.message().unwrap_or("").to_string();
                let parents: Vec<Oid> = commit.parents().map(|p| p.id()).collect();

                self.commits.push(CommitEntry {
                    oid,
                    hash: oid.to_string()[..7].to_string(),
                    author,
                    time: commit.time().seconds(),
                    subject,
                    message: body,
                    refs: labels,
                    parents,
                });
            }
        }

        self.recompute_tree();
    }

    fn recompute_tree(&mut self) {
        self.tree_lines.clear();
        let n = self.commits.len();
        if n == 0 {
            return;
        }

        let oid_to_idx: HashMap<Oid, usize> = self
            .commits
            .iter()
            .enumerate()
            .map(|(i, c)| (c.oid, i))
            .collect();

        let mut col_of = vec![0usize; n];
        let mut max_col = 0usize;

        for i in (0..n).rev() {
            let child_col = (i + 1..n).find(|&j| {
                self.commits[j]
                    .parents
                    .iter()
                    .any(|p| *p == self.commits[i].oid)
            });
            if let Some(j) = child_col {
                col_of[i] = col_of[j];
            } else {
                col_of[i] = max_col;
                max_col += 1;
            }
        }

        let mut col_min = vec![usize::MAX; max_col];
        let mut col_max = vec![0usize; max_col];
        for (i, &c) in col_of.iter().enumerate() {
            col_min[c] = col_min[c].min(i);
            col_max[c] = col_max[c].max(i);
        }

        for (i, &col) in col_of.iter().enumerate() {
            let commit = &self.commits[i];

            let has_parent = commit.parents.iter().any(|p| oid_to_idx.contains_key(p));
            let has_child = (i + 1..n)
                .any(|j| self.commits[j].parents.contains(&commit.oid));

            let mut line = String::new();
            for c in 0..max_col {
                let has_line = col_min[c] < i && col_max[c] > i;

                if c == col {
                    if has_parent && has_child {
                        line.push_str("├─");
                    } else if has_parent {
                        line.push_str("└─");
                    } else {
                        line.push_str("● ");
                    }
                } else if has_line {
                    line.push('│');
                } else {
                    line.push(' ');
                }
            }
            self.tree_lines.push(line);
        }
    }

    fn update_commit_detail(&mut self) {
        self.commit_detail.clear();
        self.scroll_offset = 0;
        if self.commits.is_empty() {
            return;
        }
        let idx = self
            .selected_commit
            .min(self.commits.len().saturating_sub(1));
        let commit = &self.commits[idx];

        let refs_str = if !commit.refs.is_empty() {
            format!(" ({})", commit.refs.join(", "))
        } else {
            String::new()
        };

        let time_str = format_relative_time(commit.time);

        let detail = format!(
            "Commit:  {}{}\nAuthor:  {}\nDate:    {} ago\n\n    {}\n\n{}",
            commit.hash,
            refs_str,
            commit.author,
            time_str,
            commit.subject,
            commit
                .message
                .lines()
                .skip(1)
                .collect::<Vec<_>>()
                .join("\n")
                .trim()
        );
        self.commit_detail = detail;
    }

    pub fn next(&mut self) {
        if self.focus == Focus::Detail {
            self.scroll_down();
            return;
        }
        match self.current_tab {
            0 | 1 => {
                if self.files.is_empty() {
                    return;
                }
                self.selected_index =
                    (self.selected_index + 1) % self.files.len();
                let _ = self.update_diff();
            }
            2 => {
                if self.commits.is_empty() {
                    return;
                }
                self.selected_commit =
                    (self.selected_commit + 1) % self.commits.len();
                self.update_commit_detail();
            }
            _ => {}
        }
    }

    pub fn previous(&mut self) {
        if self.focus == Focus::Detail {
            self.scroll_up();
            return;
        }
        match self.current_tab {
            0 | 1 => {
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
            2 => {
                if self.commits.is_empty() {
                    return;
                }
                self.selected_commit = if self.selected_commit == 0 {
                    self.commits.len() - 1
                } else {
                    self.selected_commit - 1
                };
                self.update_commit_detail();
            }
            _ => {}
        }
    }

    pub fn toggle_focus(&mut self) {
        match self.current_tab {
            0 | 1 => {
                if self.files.is_empty() {
                    return;
                }
                if self.focus == Focus::List {
                    let _ = self.update_diff();
                    self.focus = Focus::Detail;
                } else {
                    self.focus = Focus::List;
                }
            }
            2 => {
                if self.commits.is_empty() {
                    return;
                }
                if self.focus == Focus::List {
                    self.update_commit_detail();
                    self.focus = Focus::Detail;
                } else {
                    self.focus = Focus::List;
                }
            }
            _ => {}
        }
    }

    pub fn next_tab(&mut self) {
        self.current_tab = (self.current_tab + 1) % MENU_ITEMS.len();
        self.selected_index = 0;
        self.selected_commit = 0;
        self.focus = Focus::List;
        if self.current_tab == 0 || self.current_tab == 1 {
            let _ = self.update_diff();
        }
        if self.current_tab == 2 {
            self.update_commit_detail();
        }
    }

    pub fn previous_tab(&mut self) {
        self.current_tab = if self.current_tab == 0 {
            MENU_ITEMS.len() - 1
        } else {
            self.current_tab - 1
        };
        self.selected_index = 0;
        self.selected_commit = 0;
        self.focus = Focus::List;
        if self.current_tab == 0 || self.current_tab == 1 {
            let _ = self.update_diff();
        }
        if self.current_tab == 2 {
            self.update_commit_detail();
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

    fn help_text(&self) -> String {
        let focus_str = match self.focus {
            Focus::List => "List",
            Focus::Detail => "Detail",
        };
        let base = if self.focus == Focus::Detail {
            match self.current_tab {
                0 => "↑↓/PgUp/PgDn Scroll  Enter:List  q:Quit",
                1 => "↑↓/PgUp/PgDn Scroll  Enter:List  q:Quit",
                2 => "↑↓/PgUp/PgDn Scroll  Enter:List  q:Quit",
                _ => "PgUp/PgDn Scroll  Tab:Switch  q:Quit",
            }
        } else {
            match self.current_tab {
                0 => "↑↓ Files  Enter:View → Detail  Tab:Switch  q:Quit",
                1 => "↑↓ Files  Space:Stage  Enter:View → Detail  q:Quit",
                2 => "↑↓ Commits  Enter:View → Detail  Tab:Switch  q:Quit",
                _ => "Tab:Switch  q:Quit",
            }
        };
        format!("[{}] {}", focus_str, base)
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
        let widths = [12, 16, 12, 14];
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
            2 => self.render_commit_list(frame, area),
            _ => self.render_placeholder_list(frame, area),
        }
    }

    fn pane_border_style(&self, is_detail: bool) -> Style {
        let focused = if is_detail {
            self.focus == Focus::Detail
        } else {
            self.focus == Focus::List
        };
        if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        }
    }

    fn render_file_list(&self, frame: &mut Frame, area: Rect) {
        let title = format!(" {} ({}) ", self.active_menu_name(), self.files.len());
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

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(self.pane_border_style(false))
                .title(title.as_str())
                .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        );
        frame.render_widget(list, area);
    }

    fn render_commit_list(&self, frame: &mut Frame, area: Rect) {
        let title = format!(" History ({}) ", self.commits.len());
        let items: Vec<ListItem> = self
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

                let dot = if selected { "▶" } else { " " };
                let hash_color = if selected { Color::Black } else { Color::Yellow };
                let ref_str = if !commit.refs.is_empty() {
                    format!(" ({})", commit.refs.join(", "))
                } else {
                    String::new()
                };
                let time_str = format_relative_time(commit.time);

                let subject = if commit.subject.len() > 50 {
                    format!(" {}...", &commit.subject[..47])
                } else {
                    format!(" {}", commit.subject)
                };

                let spans = vec![
                    Span::styled(
                        dot.to_string(),
                        Style::default().fg(Color::Cyan).bg(bg),
                    ),
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
                .border_style(self.pane_border_style(false))
                .title(format!(" {} ", self.active_menu_name()))
                .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        );
        frame.render_widget(list, area);
    }

    fn render_right_pane(&self, frame: &mut Frame, area: Rect) {
        let title = match self.current_tab {
            0 => " Diff ",
            1 => " Stage Details ",
            2 => " Commit Details ",
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
            2 => {
                if self.commits.is_empty() {
                    Text::from(Line::from(Span::styled(
                        "  No commits found.",
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
                    let time_str = format_relative_time(commit.time);

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
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )),
            ]),
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

    fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
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
