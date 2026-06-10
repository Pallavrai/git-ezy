mod tab_bar;
mod file_list;
mod commit_list;
mod branch_list;
mod detail_pane;
mod status_bar;

use git2::Status;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    Frame,
};

use crate::app::App;
use crate::types::Focus;

impl App {
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

    fn render_left_pane(&self, frame: &mut Frame, area: Rect) {
        match self.current_tab {
            0 | 1 => self.render_file_list(frame, area),
            2 => self.render_commit_list(frame, area),
            3 => self.render_branch_list(frame, area),
            _ => {}
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
                    0 => "↑↓/PgUp/PgDn Scroll  Enter:List  ^R:Refresh  q:Quit",
                    1 => "↑↓/PgUp/PgDn Scroll  Enter:List  ^R:Refresh  q:Quit",
                    2 => "↑↓/PgUp/PgDn Scroll  Enter:List  c:Commit  ^R:Refresh  q:Quit",
                    3 => "↑↓/PgUp/PgDn Scroll  Enter:List  ^R:Refresh  q:Quit",
                    _ => "PgUp/PgDn Scroll  Tab:Switch  ^R:Refresh  q:Quit",
                }
        } else {
                match self.current_tab {
                    0 => "↑↓ Files  Enter:View → Detail  Tab:Switch  ^R:Refresh  q:Quit",
                    1 => "↑↓ Files  Space:Stage  Enter:View → Detail  ^R:Refresh  q:Quit",
                    2 => "↑↓ Commits  Type:message  Enter:Detail  c:Commit  ^R:Refresh  q:Quit",
                    3 => "↑↓ Branches  c:Checkout  Enter:View → Detail  ^R:Refresh  q:Quit",
                    _ => "Tab:Switch  ^R:Refresh  q:Quit",
                }
        };
        format!("[{}] {}", focus_str, base)
    }
}
