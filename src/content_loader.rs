// Content loader: handles SSL multi-file content loading and switching.

use std::path::{Path, PathBuf};

pub struct ContentLoader {
    pub pending_content: Option<String>,
}

impl ContentLoader {
    pub fn new() -> Self {
        Self {
            pending_content: None,
        }
    }

    /// Mark content for loading at the end of the current tick.
    pub fn queue_load(&mut self, filename: &str) {
        // Parse the filename: split by '/', trim each component, remove empty segments.
        // Native32 games often pad directory names with trailing spaces (e.g. "BBLADE  ").
        let parts: Vec<&str> = filename
            .split('/')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        if parts.is_empty() {
            log::warn!("Empty content filename");
            return;
        }
        self.pending_content = Some(parts.join("/"));
    }

    /// Check if there's pending content to load.
    pub fn has_pending(&self) -> bool {
        self.pending_content.is_some()
    }

    /// Take the pending content filename.
    pub fn take_pending(&mut self) -> Option<String> {
        self.pending_content.take()
    }

    /// Find the content file by searching up the directory tree.
    /// Handles case-insensitive matching and trailing spaces in path components.
    pub fn find_content_file(current_game_path: &Path, filename: &str) -> Option<PathBuf> {
        let relative_path = Path::new(filename);

        // Search from the current game file's directory upward
        let mut search_dir = current_game_path.parent();

        while let Some(dir) = search_dir {
            // Try exact match first
            let pattern = dir.join(relative_path);
            if pattern.exists() {
                return Some(pattern);
            }

            // Try case-insensitive match for each path component
            if let Some(resolved) = Self::fuzzy_resolve(dir, relative_path) {
                return Some(resolved);
            }

            search_dir = dir.parent();
        }

        log::warn!("Failed to find content file: {}", filename);
        None
    }

    /// Try to resolve a relative path under a base directory with fuzzy matching
    /// (case-insensitive + trailing space trimming for each component).
    fn fuzzy_resolve(base: &Path, relative: &Path) -> Option<PathBuf> {
        let mut current = base.to_path_buf();

        for component in relative.components() {
            let target = component.as_os_str().to_string_lossy().to_lowercase();
            let target = target.trim();

            let mut found = false;
            if let Ok(entries) = std::fs::read_dir(&current) {
                for entry in entries.flatten() {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy().to_lowercase();
                    if name_str.trim() == target {
                        current = entry.path();
                        found = true;
                        break;
                    }
                }
            }

            if !found {
                return None;
            }
        }

        if current.exists() {
            Some(current)
        } else {
            None
        }
    }
}
