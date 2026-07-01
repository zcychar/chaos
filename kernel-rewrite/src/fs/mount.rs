#![allow(unused_imports)]

use std::sync::RwLock;

/// One mount mapping from a path prefix to a backing target.
#[derive(Clone, Debug)]
pub struct MountEntry {
    pub prefix: String,
    pub target: String,
}

/// Ordered mount table.
///
/// Entries are sorted by descending prefix length so the longest matching
/// mount point wins during resolution.
///
/// Fix: derived a helper function to canonicalize slashes, and remove some redundant code.
/// Note: cannot make sure if resolve is correct.
pub struct MountTable {
    pub entries: RwLock<Vec<MountEntry>>,
}

impl MountTable {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(Vec::new()),
        }
    }

    pub fn bind(&self, prefix: &str, target: &str) {
        let mut entries = self.entries.write().unwrap();
        let already_bound = entries
            .iter()
            .any(|entry| entry.prefix == prefix && entry.target == target);
        if already_bound {
            return;
        }

        entries.push(MountEntry {
            prefix: prefix.to_string(),
            target: target.to_string(),
        });
        entries.sort_by(|left, right| right.prefix.len().cmp(&left.prefix.len()));
    }

    fn prefix_matches(prefix: &str, path: &str) -> bool {
        if prefix == "/" {
            return path.starts_with('/');
        }
        if !path.starts_with(prefix) {
            return false;
        }
        // Debug fix: `/mnt` must match `/mnt/file`, but not `/mnted/file`.
        path.len() == prefix.len() || path.as_bytes().get(prefix.len()) == Some(&b'/')
    }

    //remove redudant slashes.
    fn canonicalize_slashes(path: &str) -> String {
        let mut canonical = String::with_capacity(path.len());
        let mut previous_was_slash = false;
        for ch in path.chars() {
            if ch == '/' {
                if !previous_was_slash {
                    canonical.push(ch);
                }
                previous_was_slash = true;
            } else {
                canonical.push(ch);
                previous_was_slash = false;
            }
        }
        if canonical.is_empty() {
            path.to_string()
        } else {
            canonical
        }
    }

    pub fn resolve(&self, path: &str) -> Result<String, &'static str> {
        match self.find_mount(path) {
            Some(entry) => {
                let remaining_path = &path[entry.prefix.len()..];
                let resolved_suffix = self.resolve(remaining_path)?;
                let mut result =
                    String::with_capacity(entry.target.len() + 1 + resolved_suffix.len());
                result.push_str(&entry.target);
                result.push(':');
                result.push_str(&resolved_suffix);
                Ok(result)
            }
            None => Ok(Self::canonicalize_slashes(path)),
        }
    }

    pub fn unmount(&self, prefix: &str) -> bool {
        let mut entries = self.entries.write().unwrap();
        let previous_len = entries.len();
        entries.retain(|entry| entry.prefix != prefix);
        entries.len() < previous_len
    }

    pub fn list_mounts(&self) -> Vec<(String, String)> {
        let entries = self.entries.read().unwrap();
        entries
            .iter()
            .map(|entry| (entry.prefix.clone(), entry.target.clone()))
            .collect()
    }

    pub fn find_mount(&self, path: &str) -> Option<MountEntry> {
        let entries = self.entries.read().unwrap();
        let mut best_match: Option<&MountEntry> = None;
        let mut best_prefix_len = 0usize;

        for entry in entries.iter() {
            let prefix_len = entry.prefix.len();
            if prefix_len == 0 {
                continue;
            }
            if Self::prefix_matches(&entry.prefix, path) && prefix_len > best_prefix_len {
                best_prefix_len = prefix_len;
                best_match = Some(entry);
            }
        }

        best_match.cloned()
    }

    pub fn mount_count(&self) -> usize {
        self.entries.read().unwrap().len()
    }

    pub fn has_prefix(&self, prefix: &str) -> bool {
        self.entries
            .read()
            .unwrap()
            .iter()
            .any(|entry| entry.prefix.as_bytes() == prefix.as_bytes())
    }
}
