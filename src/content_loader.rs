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
        // Parse the filename: split by '/', remove empty segments
        let parts: Vec<&str> = filename.split('/').filter(|s| !s.trim().is_empty()).collect();
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
    pub fn find_content_file(current_game_path: &Path, filename: &str) -> Option<PathBuf> {
        let relative_path = Path::new(filename);

        // Search from the current game file's directory upward
        let mut search_dir = current_game_path.parent();

        while let Some(dir) = search_dir {
            // Try case-insensitive matching using glob
            let pattern = dir.join(relative_path);

            // Try exact match first
            if pattern.exists() {
                return Some(pattern);
            }

            // Try case-insensitive glob match
            if let Some(parent) = pattern.parent() {
                if let Some(file_name) = pattern.file_name() {
                    let file_name_str = file_name.to_string_lossy().to_lowercase();
                    if let Ok(entries) = std::fs::read_dir(parent) {
                        for entry in entries.flatten() {
                            if entry.file_name().to_string_lossy().to_lowercase() == file_name_str {
                                return Some(entry.path());
                            }
                        }
                    }
                }
            }

            search_dir = dir.parent();
        }

        log::warn!("Failed to find content file: {}", filename);
        None
    }
}
