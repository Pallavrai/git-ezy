use std::collections::HashMap;

use crate::app::App;

impl App {
    pub fn recompute_tree(&mut self) {
        self.tree_lines.clear();
        let n = self.commits.len();
        if n == 0 {
            return;
        }

        let oid_to_idx: HashMap<_, _> = self
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

        if max_col <= 1 {
            for _ in &self.commits {
                self.tree_lines.push("● ".to_string());
            }
            return;
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
}
