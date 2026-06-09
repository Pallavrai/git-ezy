use git2::{Oid, Status};

pub const MENU_ITEMS: &[&str] = &["Status", "Stage Files", "Commit", "Branches"];
pub const SCROLL_STEP: usize = 8;

#[derive(Debug, Clone)]
pub struct FileItem {
    pub path: String,
    pub status: Status,
}

#[derive(Debug, Clone)]
pub struct BranchEntry {
    pub name: String,
    pub is_current: bool,
    pub oid: Oid,
    pub upstream: Option<String>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    List,
    Detail,
}

pub fn format_relative_time(seconds: i64) -> String {
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
