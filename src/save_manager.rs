// Save manager: handles persistence of save data to .ssl_sav files.

use std::path::{Path, PathBuf};

pub struct SaveManager {
    save_path: PathBuf,
}

impl SaveManager {
    pub fn new(game_path: &Path) -> Self {
        let save_path = PathBuf::from(format!("{}.ssl_sav", game_path.display()));
        Self { save_path }
    }

    /// Save data to the .ssl_sav file.
    pub fn save(&self, data: &str) -> bool {
        match std::fs::write(&self.save_path, data) {
            Ok(()) => {
                log::info!("Saved data to {}", self.save_path.display());
                true
            }
            Err(e) => {
                log::error!("Failed to save data: {}", e);
                false
            }
        }
    }

    /// Load data from the .ssl_sav file.
    /// Returns Some(data) if the file exists, None otherwise.
    pub fn load(&self) -> Option<String> {
        if !self.save_path.exists() {
            log::info!("Save file not found: {}", self.save_path.display());
            return None;
        }
        match std::fs::read_to_string(&self.save_path) {
            Ok(data) => {
                log::info!("Loaded save data from {}", self.save_path.display());
                Some(data)
            }
            Err(e) => {
                log::error!("Failed to load save data: {}", e);
                None
            }
        }
    }
}
