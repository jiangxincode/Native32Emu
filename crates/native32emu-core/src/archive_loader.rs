// Archive loader for ZIP game packages.
//
// Native32 games are distributed as ZIP archives containing a complete game
// directory structure with FHUI.smf (main menu) and game subdirectories.
// This module handles extraction and locating the entry-point SMF file.

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Extract a ZIP archive to a temporary directory and return the path.
///
/// The `TempDir` handle is returned alongside the path so the caller can keep
/// it alive — the directory is automatically deleted when the handle is dropped.
pub fn extract_zip(zip_path: &Path) -> Result<(tempfile::TempDir, PathBuf)> {
    let file = fs::File::open(zip_path)
        .with_context(|| format!("Failed to open ZIP file: {}", zip_path.display()))?;

    let mut archive = zip::ZipArchive::new(file)
        .with_context(|| format!("Failed to read ZIP archive: {}", zip_path.display()))?;

    // Create a temporary directory for extraction
    let temp_dir =
        tempfile::tempdir().context("Failed to create temporary directory for ZIP extraction")?;
    let extract_path = temp_dir.path().to_path_buf();

    // Extract all files from the archive
    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .with_context(|| format!("Failed to read ZIP entry at index {}", i))?;

        let outpath = extract_path.join(file.mangled_name());

        // Skip directories (they'll be created as needed)
        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath)
                .with_context(|| format!("Failed to create directory: {}", outpath.display()))?;
            continue;
        }

        // Create parent directories if needed
        if let Some(parent) = outpath.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).with_context(|| {
                    format!("Failed to create parent directory: {}", parent.display())
                })?;
            }
        }

        // Extract the file
        let mut outfile = fs::File::create(&outpath)
            .with_context(|| format!("Failed to create file: {}", outpath.display()))?;
        std::io::copy(&mut file, &mut outfile)
            .with_context(|| format!("Failed to extract file: {}", outpath.display()))?;
    }

    Ok((temp_dir, extract_path))
}

/// Find the FHUI.smf (main menu) file in an extracted directory.
///
/// Searches for FHUI.smf in the root of the extracted directory, handling
/// case-insensitive matching. Returns the path if found, or None if not found.
pub fn find_fhui_in_directory(dir: &Path) -> Option<PathBuf> {
    // Try exact match first
    let fhui_path = dir.join("FHUI.smf");
    if fhui_path.exists() {
        return Some(fhui_path);
    }

    // Try case-insensitive match
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.eq_ignore_ascii_case("FHUI.smf") {
                return Some(entry.path());
            }
        }
    }

    None
}

/// Process a ZIP file: extract it and find the FHUI.smf entry point.
///
/// Returns the `TempDir` handle (keep it alive to preserve the directory) and
/// the path to the extracted FHUI.smf file. The directory is automatically
/// deleted when the `TempDir` is dropped.
pub fn load_zip_game(zip_path: &Path) -> Result<(tempfile::TempDir, PathBuf)> {
    log::info!("Extracting ZIP archive: {}", zip_path.display());

    let (temp_dir, extract_path) = extract_zip(zip_path)?;

    // Find FHUI.smf in the extracted directory
    match find_fhui_in_directory(&extract_path) {
        Some(fhui_path) => {
            log::info!("Found FHUI.smf: {}", fhui_path.display());
            Ok((temp_dir, fhui_path))
        }
        None => {
            // temp_dir is dropped here, automatically cleaning up the directory
            anyhow::bail!("No FHUI.smf found in ZIP archive: {}", zip_path.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_extract_zip_empty_archive() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("test.zip");

        // Create an empty ZIP file
        let file = fs::File::create(&zip_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.finish().unwrap();

        let result = extract_zip(&zip_path);
        assert!(result.is_ok());
        // TempDir cleans up automatically when dropped
    }

    #[test]
    fn test_extract_zip_with_fhui() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("test.zip");

        // Create a ZIP with FHUI.smf
        let file = fs::File::create(&zip_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);

        zip.start_file("FHUI.smf", options).unwrap();
        zip.write_all(b"fake smf content").unwrap();
        zip.finish().unwrap();

        let result = extract_zip(&zip_path);
        assert!(result.is_ok());

        let (_temp_dir, extract_path) = result.unwrap();
        assert!(extract_path.join("FHUI.smf").exists());
        // TempDir cleans up automatically when dropped
    }

    #[test]
    fn test_find_fhui_exact_match() {
        let dir = tempfile::tempdir().unwrap();
        let fhui_path = dir.path().join("FHUI.smf");
        fs::write(&fhui_path, b"fake").unwrap();

        let result = find_fhui_in_directory(dir.path());
        assert_eq!(result, Some(fhui_path));
    }

    #[test]
    fn test_find_fhui_case_insensitive() {
        let dir = tempfile::tempdir().unwrap();
        let fhui_path = dir.path().join("fhui.smf");
        fs::write(&fhui_path, b"fake").unwrap();

        let result = find_fhui_in_directory(dir.path());
        assert!(result.is_some());
        // The path should exist and have the correct filename (case may vary)
        let result_path = result.unwrap();
        assert!(result_path.exists());
        assert_eq!(
            result_path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_lowercase(),
            "fhui.smf"
        );
    }

    #[test]
    fn test_find_fhui_not_found() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("other.smf"), b"fake").unwrap();

        let result = find_fhui_in_directory(dir.path());
        assert!(result.is_none());
    }

    #[test]
    fn test_load_zip_game_with_fhui() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("game.zip");

        // Create a ZIP with FHUI.smf
        let file = fs::File::create(&zip_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);

        zip.start_file("FHUI.smf", options).unwrap();
        zip.write_all(b"fake smf content").unwrap();
        zip.finish().unwrap();

        let result = load_zip_game(&zip_path);
        assert!(result.is_ok());

        let (_temp_dir, fhui_path) = result.unwrap();
        assert!(fhui_path.exists());
        assert_eq!(fhui_path.file_name().unwrap().to_str().unwrap(), "FHUI.smf");
        // TempDir cleans up automatically when dropped
    }

    #[test]
    fn test_load_zip_game_no_fhui() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("game.zip");

        // Create a ZIP without FHUI.smf
        let file = fs::File::create(&zip_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);

        zip.start_file("other.smf", options).unwrap();
        zip.write_all(b"fake smf content").unwrap();
        zip.finish().unwrap();

        let result = load_zip_game(&zip_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_zip_game_with_subdirectory() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("game.zip");

        // Create a ZIP with nested structure (FHUI.smf in root, not in subdirectory)
        let file = fs::File::create(&zip_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);

        // Add FHUI.smf in root (typical structure)
        zip.start_file("FHUI.smf", options).unwrap();
        zip.write_all(b"fake smf content").unwrap();
        // Add a subdirectory with game files
        zip.add_directory("EACT/", options).unwrap();
        zip.start_file("EACT/GAME.smf", options).unwrap();
        zip.write_all(b"fake game content").unwrap();
        zip.finish().unwrap();

        let result = load_zip_game(&zip_path);
        assert!(result.is_ok());
        // TempDir cleans up automatically when dropped
    }
}
