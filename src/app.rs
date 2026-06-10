use git2::Repository;

use crate::types::{BranchEntry, CommitEntry, FileItem, Focus, MENU_ITEMS, SCROLL_STEP};

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
    pub branches: Vec<BranchEntry>,
    pub selected_branch: usize,
    pub scroll_offset: usize,
    pub list_scroll_offset: usize,
    pub h_scroll_offset: usize,
    pub focus: Focus,
    pub commit_message: String,
    pub commit_mode: bool,
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
            branches: Vec::new(),
            selected_branch: 0,
            scroll_offset: 0,
            list_scroll_offset: 0,
            h_scroll_offset: 0,
            focus: Focus::List,
            commit_message: String::new(),
            commit_mode: false,
        };
        app.refresh_status();
        app.load_commits();
        app.load_branches();
        app
    }

    pub fn active_menu_name(&self) -> &str {
        MENU_ITEMS[self.current_tab]
    }

    pub fn update_commit_detail(&mut self) {
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

        let time_str = crate::types::format_relative_time(commit.time);

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
                self.list_scroll_offset = self.selected_commit;
                self.update_commit_detail();
            }
            3 => {
                if self.branches.is_empty() {
                    return;
                }
                self.selected_branch =
                    (self.selected_branch + 1) % self.branches.len();
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
                self.list_scroll_offset = self.selected_commit;
                self.update_commit_detail();
            }
            3 => {
                if self.branches.is_empty() {
                    return;
                }
                self.selected_branch = if self.selected_branch == 0 {
                    self.branches.len() - 1
                } else {
                    self.selected_branch - 1
                };
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
            3 => {
                if self.branches.is_empty() {
                    return;
                }
                if self.focus == Focus::List {
                    self.scroll_offset = 0;
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
        self.selected_branch = 0;
        self.list_scroll_offset = 0;
        self.h_scroll_offset = 0;
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
        self.selected_branch = 0;
        self.list_scroll_offset = 0;
        self.h_scroll_offset = 0;
        self.focus = Focus::List;
        if self.current_tab == 0 || self.current_tab == 1 {
            let _ = self.update_diff();
        }
        if self.current_tab == 2 {
            self.update_commit_detail();
        }
    }

    pub fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(SCROLL_STEP);
    }

    pub fn refresh(&mut self) {
        self.repo = git2::Repository::open(".").ok();
        self.branch = self
            .repo
            .as_ref()
            .and_then(|r| r.head().ok())
            .and_then(|h| h.shorthand().map(String::from))
            .unwrap_or_else(|| "detached".into());
        self.refresh_status();
        self.load_commits();
        self.load_branches();
        if self.current_tab == 0 || self.current_tab == 1 {
            if self.focus == Focus::Detail {
                let _ = self.update_diff();
            }
        }
        if self.current_tab == 2 {
            self.update_commit_detail();
        }
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(SCROLL_STEP);
    }

    pub fn scroll_right(&mut self) {
        self.h_scroll_offset = self.h_scroll_offset.saturating_add(SCROLL_STEP);
    }

    pub fn scroll_left(&mut self) {
        self.h_scroll_offset = self.h_scroll_offset.saturating_sub(SCROLL_STEP);
    }
}
