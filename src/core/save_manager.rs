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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_path_has_ssl_sav_suffix() {
        let game_path = Path::new("/some/path/game.ssl");
        let sm = SaveManager::new(game_path);
        assert_eq!(sm.save_path, PathBuf::from("/some/path/game.ssl.ssl_sav"));
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let game_path = dir.path().join("test_game.ssl");
        std::fs::write(&game_path, b"fake game data").unwrap();

        let sm = SaveManager::new(&game_path);
        let test_data = "highscore=9999;level=5";

        assert!(sm.save(test_data));
        let loaded = sm.load();
        assert_eq!(loaded, Some(test_data.to_string()));
    }

    #[test]
    fn test_load_nonexistent_returns_none() {
        let sm = SaveManager::new(Path::new("/nonexistent/path/game.ssl"));
        assert!(sm.load().is_none());
    }

    #[test]
    fn test_save_overwrites_existing() {
        let dir = tempfile::tempdir().unwrap();
        let game_path = dir.path().join("test_game.ssl");
        std::fs::write(&game_path, b"fake").unwrap();

        let sm = SaveManager::new(&game_path);
        assert!(sm.save("first"));
        assert!(sm.save("second"));
        assert_eq!(sm.load(), Some("second".to_string()));
    }

    #[test]
    fn test_save_returns_true_on_success() {
        let dir = tempfile::tempdir().unwrap();
        let game_path = dir.path().join("test_game.ssl");
        std::fs::write(&game_path, b"fake").unwrap();

        let sm = SaveManager::new(&game_path);
        assert!(sm.save("data"));
    }

    #[test]
    fn test_load_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let game_path = dir.path().join("test_game.ssl");
        std::fs::write(&game_path, b"fake").unwrap();

        let sm = SaveManager::new(&game_path);
        sm.save("");
        assert_eq!(sm.load(), Some("".to_string()));
    }

    #[test]
    fn test_save_unicode_data() {
        let dir = tempfile::tempdir().unwrap();
        let game_path = dir.path().join("test_game.ssl");
        std::fs::write(&game_path, b"fake").unwrap();

        let sm = SaveManager::new(&game_path);
        let data = "name=测试玩家;score=100";
        assert!(sm.save(data));
        assert_eq!(sm.load(), Some(data.to_string()));
    }
}
