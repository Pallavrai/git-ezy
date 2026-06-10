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

        for i in 0..n {
            let child = (0..i).find(|&j| {
                self.commits[j]
                    .parents
                    .contains(&self.commits[i].oid)
            });
            if let Some(j) = child {
                col_of[i] = col_of[j];
            } else {
                col_of[i] = max_col;
                max_col += 1;
            }
        }

        for (i, &col) in col_of.iter().enumerate() {
            let has_parent = self.commits[i]
                .parents
                .iter()
                .any(|p| oid_to_idx.contains_key(p));
            let has_child = (0..i)
                .any(|j| self.commits[j].parents.contains(&self.commits[i].oid));

            let mut line = String::new();
            for c in 0..max_col {
                let above = (0..i).any(|k| col_of[k] == c);
                let below = (i + 1..n).any(|k| col_of[k] == c);

                if c == col {
                    if has_parent && has_child {
                        line.push_str("├─");
                    } else if has_parent {
                        line.push_str("└─");
                    } else if has_child {
                        line.push_str("●─");
                    } else {
                        line.push_str("● ");
                    }
                } else if above && below {
                    line.push_str("│ ");
                } else {
                    line.push_str("  ");
                }
            }
            self.tree_lines.push(line);
        }
    }
}
