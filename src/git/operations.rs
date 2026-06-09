use std::collections::HashMap;

use anyhow::Result;
use git2::{DiffOptions, Oid, Status};

use crate::app::App;
use crate::types::BranchEntry;

impl App {
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
                self.files.push(crate::types::FileItem { path, status });
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

        if is_staged {
            let restored = {
                if let Ok(tree) = repo.head().and_then(|h| h.peel_to_tree()) {
                    if let Ok(tree_entry) = tree.get_path(path) {
                        if let Ok(obj) = tree_entry.to_object(repo) {
                            if let Ok(blob) = obj.peel_to_blob() {
                                let mut index = repo.index()?;
                                let entry = git2::IndexEntry {
                                    ctime: git2::IndexTime::new(0, 0),
                                    mtime: git2::IndexTime::new(0, 0),
                                    dev: 0,
                                    ino: 0,
                                    mode: tree_entry.filemode() as u32,
                                    uid: 0,
                                    gid: 0,
                                    file_size: 0,
                                    id: blob.id(),
                                    flags: 0,
                                    flags_extended: 0,
                                    path: path.to_str().unwrap_or("").to_string().into_bytes(),
                                };
                                index.add_frombuffer(&entry, blob.content())?;
                                index.write()?;
                                true
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            };
            if !restored {
                let mut index = repo.index()?;
                index.remove_path(path)?;
                index.write()?;
            }
        } else if item.status == Status::WT_DELETED {
            let mut index = repo.index()?;
            index.remove_path(path)?;
            index.write()?;
        } else {
            let mut index = repo.index()?;
            index.add_path(path)?;
            index.write()?;
        }
        self.refresh_status();
        self.update_diff()?;
        Ok(())
    }

    pub fn load_branches(&mut self) {
        self.branches.clear();
        let repo = match &self.repo {
            Some(r) => r,
            None => return,
        };
        let current = self.branch.clone();
        let branches = match repo.branches(Some(git2::BranchType::Local)) {
            Ok(b) => b,
            Err(_) => return,
        };
        for branch in branches.flatten() {
            let (branch_ref, _) = branch;
            let name = match branch_ref.name() {
                Ok(Some(n)) => n.to_string(),
                _ => continue,
            };
            let oid = match branch_ref.get().peel_to_commit() {
                Ok(c) => c.id(),
                Err(_) => continue,
            };
            let is_current = name == current;
            let upstream = match branch_ref.upstream() {
                Ok(b) => match b.name() {
                    Ok(Some(n)) => Some(n.to_string()),
                    _ => None,
                },
                Err(_) => None,
            };
            self.branches.push(BranchEntry {
                name,
                is_current,
                oid,
                upstream,
            });
        }
        self.branches.sort_by(|a, _| {
            if a.is_current {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            }
        });
        self.selected_branch = self
            .selected_branch
            .min(self.branches.len().saturating_sub(1));
    }

    pub fn checkout_branch(&mut self, name: &str) -> Result<()> {
        let rev = format!("refs/heads/{name}");
        {
            let repo = self.repo.as_ref().unwrap();
            let obj = repo.revparse_single(&rev)?;
            repo.checkout_tree(&obj, None)?;
            repo.set_head(&rev)?;
        }
        self.branch = name.to_string();
        self.load_branches();
        self.refresh_status();
        self.load_commits();
        Ok(())
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

    pub fn load_commits(&mut self) {
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
        revwalk
            .set_sorting(git2::Sort::TIME | git2::Sort::TOPOLOGICAL)
            .ok();

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

                self.commits.push(crate::types::CommitEntry {
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

    pub fn perform_commit(&mut self) -> Result<()> {
        let msg = self.commit_message.trim().to_string();
        if msg.is_empty() {
            return Ok(());
        }
        let has_staged = self.files.iter().any(|f| {
            f.status.intersects(
                Status::INDEX_NEW | Status::INDEX_MODIFIED | Status::INDEX_DELETED,
            )
        });
        if !has_staged {
            return Ok(());
        }
        if let Some(repo) = &self.repo {
            let head = repo.head()?;
            let parent = head.peel_to_commit()?;
            let tree_oid = repo.index()?.write_tree()?;
            let tree = repo.find_tree(tree_oid)?;
            let signature = repo.signature()?;
            repo.commit(
                Some("HEAD"),
                &signature,
                &signature,
                &msg,
                &tree,
                &[&parent],
            )?;
        }
        self.commit_message.clear();
        self.refresh_status();
        self.load_commits();
        Ok(())
    }
}
