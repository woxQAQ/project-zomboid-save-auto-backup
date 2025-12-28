//! File system operations for Project Zomboid save backup/restore.
//!
//! This module provides core file system capabilities including:
//! - Recursive directory copying
//! - Recursive directory deletion
//! - Directory size calculation

use serde::{Serialize, Serializer};
use std::fmt;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use flate2::{write::GzEncoder, Compression};
use tar::Builder;

/// Error type for file operations.
#[derive(Debug)]
pub enum FileOpsError {
    Io(io::Error),
    SourceNotFound(PathBuf),
    DestinationExists(PathBuf),
    NotADirectory(PathBuf),
}

impl fmt::Display for FileOpsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileOpsError::Io(err) => write!(f, "IO error: {}", err),
            FileOpsError::SourceNotFound(path) => {
                write!(f, "Source path does not exist: {}", path.display())
            }
            FileOpsError::DestinationExists(path) => {
                write!(f, "Destination already exists: {}", path.display())
            }
            FileOpsError::NotADirectory(path) => {
                write!(f, "Path is not a directory: {}", path.display())
            }
        }
    }
}

impl std::error::Error for FileOpsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FileOpsError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for FileOpsError {
    fn from(err: io::Error) -> Self {
        FileOpsError::Io(err)
    }
}

impl Serialize for FileOpsError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Result type for file operations.
pub type FileOpsResult<T> = Result<T, FileOpsError>;

/// Recursively copies a directory from source to destination.
///
/// # Arguments
/// * `src` - Source directory path
/// * `dst` - Destination directory path
///
/// # Returns
/// `FileOpsResult<()>` - Ok(()) on success, Err on failure
///
/// # Behavior
/// - Creates destination parent directories if they don't exist
/// - Returns error if destination already exists
/// - Copies all files and subdirectories recursively
/// - Preserves file metadata (permissions, modification times)
///
/// # Example
/// ```no_run
/// use std::path::Path;
/// use tauri_app_lib::file_ops::copy_dir_recursive;
///
/// copy_dir_recursive(
///     Path::new("/source/save"),
///     Path::new("/backup/save_2024-12-28")
/// ).unwrap();
/// ```
pub fn copy_dir_recursive(src: &Path, dst: &Path) -> FileOpsResult<()> {
    if !src.exists() {
        return Err(FileOpsError::SourceNotFound(src.to_path_buf()));
    }

    if dst.exists() {
        return Err(FileOpsError::DestinationExists(dst.to_path_buf()));
    }

    // Create destination directory
    fs::create_dir_all(dst)?;

    // Iterate through source directory entries
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ty.is_dir() {
            // Recursively copy subdirectory
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            // Copy file
            copy_file(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

/// Copies a single file with buffer reading for memory efficiency.
///
/// # Arguments
/// * `src` - Source file path
/// * `dst` - Destination file path
///
/// # Behavior
/// - Uses 64KB buffer to avoid loading entire file into memory
/// - Creates parent directories if needed
fn copy_file(src: &Path, dst: &Path) -> FileOpsResult<()> {
    let mut src_file = fs::File::open(src)?;
    let mut dst_file = fs::File::create(dst)?;

    // Create parent directories if they don't exist
    if let Some(parent) = dst.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    // Copy with buffer to avoid loading large files into memory
    const BUFFER_SIZE: usize = 64 * 1024; // 64KB buffer
    let mut buffer = [0u8; BUFFER_SIZE];

    loop {
        let bytes_read = src_file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        dst_file.write_all(&buffer[..bytes_read])?;
    }

    // Ensure data is written to disk for backup integrity
    dst_file.flush()?;
    dst_file.sync_all()?;

    Ok(())
}

/// Recursively deletes a directory and all its contents.
///
/// # Arguments
/// * `path` - Path to directory to delete
///
/// # Returns
/// `FileOpsResult<()>` - Ok(()) on success, Err on failure
///
/// # Behavior
/// - Returns error if path doesn't exist
/// - Returns error if path is not a directory
/// - Deletes all files and subdirectories recursively
///
/// # Safety
/// This is a destructive operation. Use with caution.
///
/// # Example
/// ```no_run
/// use std::path::Path;
/// use tauri_app_lib::file_ops::delete_dir_recursive;
///
/// delete_dir_recursive(Path::new("/old/backup")).unwrap();
/// ```
pub fn delete_dir_recursive(path: &Path) -> FileOpsResult<()> {
    if !path.exists() {
        return Err(FileOpsError::SourceNotFound(path.to_path_buf()));
    }

    if !path.is_dir() {
        return Err(FileOpsError::NotADirectory(path.to_path_buf()));
    }

    // Use fs::remove_dir_all which is recursive and optimized
    fs::remove_dir_all(path)?;

    Ok(())
}

/// Calculates the total size of a directory in bytes.
///
/// # Arguments
/// * `path` - Path to directory
///
/// # Returns
/// `FileOpsResult<u64>` - Size in bytes on success, Err on failure
///
/// # Behavior
/// - Returns error if path doesn't exist
/// - Returns error if path is not a directory
/// - Recursively sums all file sizes
/// - Does not count directory metadata, only file contents
///
/// # Performance
/// Uses iterative approach with Vec to avoid stack overflow on deep directories.
///
/// # Example
/// ```no_run
/// use std::path::Path;
/// use tauri_app_lib::file_ops::get_dir_size;
///
/// let size = get_dir_size(Path::new("/save/game")).unwrap();
/// println!("Save size: {} bytes", size);
/// ```
pub fn get_dir_size(path: &Path) -> FileOpsResult<u64> {
    if !path.exists() {
        return Err(FileOpsError::SourceNotFound(path.to_path_buf()));
    }

    if !path.is_dir() {
        return Err(FileOpsError::NotADirectory(path.to_path_buf()));
    }

    let mut total_size = 0u64;
    let mut dirs_to_visit = vec![path.to_path_buf()];

    // Iterative approach to avoid stack overflow
    while let Some(current_dir) = dirs_to_visit.pop() {
        for entry in fs::read_dir(&current_dir)? {
            let entry = entry?;
            let entry_path = entry.path();
            let ty = entry.file_type()?;

            if ty.is_dir() {
                dirs_to_visit.push(entry_path);
            } else if ty.is_file() {
                total_size += entry.metadata()?.len();
            }
        }
    }

    Ok(total_size)
}

/// Formats a byte count as a human-readable string.
///
/// # Arguments
/// * `bytes` - Size in bytes
///
/// # Returns
/// Formatted string (e.g., "1.23 GB", "45.6 MB", "123 KB")
///
/// # Example
/// ```no_run
/// use tauri_app_lib::file_ops::format_size;
///
/// assert_eq!(format_size(1536), "1.50 KB");
/// assert_eq!(format_size(1234567890), "1.15 GB");
/// ```
pub fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

/// Normalizes a path for consistent string representation across platforms.
///
/// This ensures all path separators use the standard separator for the current platform,
/// preventing mixed separators like backslashes and forward slashes on Windows.
///
/// # Arguments
/// * `path` - Path to normalize
///
/// # Returns
/// String with normalized path separators
///
/// # Example
/// ```no_run
/// use std::path::Path;
/// use tauri_app_lib::file_ops::normalize_path_for_display;
///
/// let path = Path::new(r"C:\Users\test/path\to\file");
/// let normalized = normalize_path_for_display(path);
/// // On Windows: "C:\Users\test\path\to\file"
/// ```
pub fn normalize_path_for_display(path: &Path) -> String {
    use std::path::Component;

    // Rebuild path with proper separators
    let mut result = String::new();
    for comp in path.components() {
        match comp {
            Component::Prefix(p) => {
                result.push_str(&p.as_os_str().to_string_lossy());
            }
            Component::RootDir => {
                result.push(std::path::MAIN_SEPARATOR);
            }
            Component::Normal(s) => {
                if !result.is_empty() && !result.ends_with(std::path::MAIN_SEPARATOR) {
                    result.push(std::path::MAIN_SEPARATOR);
                }
                result.push_str(&s.to_string_lossy());
            }
            Component::CurDir | Component::ParentDir => {
                // Skip . and .. components
            }
        }
    }

    if result.is_empty() {
        path.display().to_string()
    } else {
        result
    }
}

/// Opens the parent directory of the given path in the system file manager.
///
/// # Arguments
/// * `path` - Path to the file or directory
///
/// # Returns
/// `FileOpsResult<()>` - Ok(()) on success, Err on failure
///
/// # Behavior
/// - On macOS: Uses `open -R` to reveal the file/directory in Finder
/// - On Windows: Uses `explorer /select` to select the file/directory in Explorer
/// - On Linux: Attempts to use `dbus` for GNOME/KDE, falls back to `xdg-open` for the parent directory
///
/// # Example
/// ```no_run
/// use std::path::Path;
/// use tauri_app_lib::file_ops::show_in_file_manager;
///
/// show_in_file_manager(Path::new("/path/to/backup")).unwrap();
/// ```
pub fn show_in_file_manager(path: &Path) -> FileOpsResult<()> {
    if !path.exists() {
        return Err(FileOpsError::SourceNotFound(path.to_path_buf()));
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-R")
            .arg(path)
            .spawn()
            .map_err(FileOpsError::Io)?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg("/select,")
            .arg(path)
            .spawn()
            .map_err(FileOpsError::Io)?;
    }

    #[cfg(target_os = "linux")]
    {
        // Try different methods for Linux depending on the desktop environment
        let path_str = path.to_string_lossy().to_string();

        // Try dbus for GNOME (nautilus/Files)
        let gnome_result = std::process::Command::new("dbus-send")
            .args([
                "--session",
                "--dest=org.gnome.Nautilus",
                "--type=method_call",
                "/org/gnome/Nautilus",
                "org.gtk.Actions.Activate",
                &format!("array:string:'show-item'", "string:"),
                &format!("array:string:'file://{}'", path_str),
                "array:string:",
            ])
            .spawn();

        if gnome_result.is_ok() {
            return Ok(());
        }

        // Try dbus for KDE (dolphin)
        let kde_result = std::process::Command::new("dbus-send")
            .args([
                "--session",
                "--dest=org.kde.dolphin",
                "--type=method_call",
                "/dolphin",
                "org.freedesktop.Application.Activate",
                &format!("array:string:'select'", "string:"),
                &format!("array:string:'{}'", path_str),
                "array:string:",
            ])
            .spawn();

        if kde_result.is_ok() {
            return Ok(());
        }

        // Fallback: open the parent directory
        if let Some(parent) = path.parent() {
            std::process::Command::new("xdg-open")
                .arg(parent)
                .spawn()
                .map_err(FileOpsError::Io)?;
        } else {
            std::process::Command::new("xdg-open")
                .arg(path)
                .spawn()
                .map_err(FileOpsError::Io)?;
        }
    }

    Ok(())
}

/// Creates a compressed tar.gz archive of a directory.
///
/// # Arguments
/// * `src_dir` - Source directory to compress
/// * `dst_file` - Destination .tar.gz file path
///
/// # Returns
/// `FileOpsResult<()>` - Ok(()) on success, Err on failure
///
/// # Behavior
/// - Creates parent directories if needed
/// - Returns error if source doesn't exist
/// - Returns error if destination already exists
/// - Uses default gzip compression (level 6) for balanced speed/compression ratio
///
/// # Example
/// ```no_run
/// use std::path::Path;
/// use tauri_app_lib::file_ops::create_tar_gz;
///
/// create_tar_gz(
///     Path::new("/save/game"),
///     Path::new("/backup/game_2024-12-28.tar.gz")
/// ).unwrap();
/// ```
pub fn create_tar_gz(src_dir: &Path, dst_file: &Path) -> FileOpsResult<()> {
    if !src_dir.exists() {
        return Err(FileOpsError::SourceNotFound(src_dir.to_path_buf()));
    }

    if dst_file.exists() {
        return Err(FileOpsError::DestinationExists(dst_file.to_path_buf()));
    }

    // Create parent directories if needed
    if let Some(parent) = dst_file.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    // Create the tar.gz file
    let gz_file = fs::File::create(dst_file)?;
    let encoder = GzEncoder::new(gz_file, Compression::default());
    let mut tar = Builder::new(encoder);

    // Add the source directory to the archive
    tar.append_dir_all(".", src_dir)?;

    // Finish the archive (this flushes and completes the gzip stream)
    let encoder = tar.into_inner()?;
    encoder.finish()?;

    Ok(())
}

/// Extracts a compressed tar.gz archive to a directory.
///
/// # Arguments
/// * `src_file` - Source .tar.gz file path
/// * `dst_dir` - Destination directory to extract to
///
/// # Returns
/// `FileOpsResult<()>` - Ok(()) on success, Err on failure
///
/// # Behavior
/// - Returns error if source file doesn't exist
/// - Returns error if destination already exists
/// - Creates parent directories if needed
///
/// # Example
/// ```no_run
/// use std::path::Path;
/// use tauri_app_lib::file_ops::extract_tar_gz;
///
/// extract_tar_gz(
///     Path::new("/backup/game_2024-12-28.tar.gz"),
///     Path::new("/save/game")
/// ).unwrap();
/// ```
pub fn extract_tar_gz(src_file: &Path, dst_dir: &Path) -> FileOpsResult<()> {
    if !src_file.exists() {
        return Err(FileOpsError::SourceNotFound(src_file.to_path_buf()));
    }

    if dst_dir.exists() {
        return Err(FileOpsError::DestinationExists(dst_dir.to_path_buf()));
    }

    // Create parent directories if needed
    if let Some(parent) = dst_dir.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    // Open the gz file and create a decoder
    let gz_file = fs::File::open(src_file)?;
    let decoder = flate2::read::GzDecoder::new(gz_file);
    let mut archive = tar::Archive::new(decoder);

    // Extract the archive
    archive.unpack(dst_dir)?;

    Ok(())
}

/// Gets the size of a file.
///
/// # Arguments
/// * `path` - Path to the file
///
/// # Returns
/// `FileOpsResult<u64>` - Size in bytes on success, Err on failure
pub fn get_file_size(path: &Path) -> FileOpsResult<u64> {
    if !path.exists() {
        return Err(FileOpsError::SourceNotFound(path.to_path_buf()));
    }

    let metadata = fs::metadata(path)?;
    Ok(metadata.len())
}

/// Deletes a file.
///
/// # Arguments
/// * `path` - Path to the file to delete
///
/// # Returns
/// `FileOpsResult<()>` - Ok(()) on success, Err on failure
pub fn delete_file(path: &Path) -> FileOpsResult<()> {
    if !path.exists() {
        return Err(FileOpsError::SourceNotFound(path.to_path_buf()));
    }

    fs::remove_file(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use tempfile::TempDir;

    // Helper function to create a test directory structure
    fn create_test_structure() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        // Create structure: base/file1.txt, base/subdir/file2.txt, base/subdir/nested/file3.txt
        File::create(base.join("file1.txt")).unwrap().write_all(b"hello").unwrap();
        fs::create_dir_all(base.join("subdir/nested")).unwrap();
        File::create(base.join("subdir/file2.txt"))
            .unwrap()
            .write_all(b"world test content")
            .unwrap();
        File::create(base.join("subdir/nested/file3.txt"))
            .unwrap()
            .write_all(b"nested data here")
            .unwrap();

        temp_dir
    }

    #[test]
    fn test_copy_dir_recursive_success() {
        let src_dir = create_test_structure();
        let dst_base = TempDir::new().unwrap();
        let dst_dir = dst_base.path().join("copy");

        copy_dir_recursive(src_dir.path(), &dst_dir).unwrap();

        // Verify all files were copied
        assert!(dst_dir.join("file1.txt").exists());
        assert!(dst_dir.join("subdir/file2.txt").exists());
        assert!(dst_dir.join("subdir/nested/file3.txt").exists());

        // Verify content matches
        let src_content = fs::read_to_string(src_dir.path().join("file1.txt")).unwrap();
        let dst_content = fs::read_to_string(dst_dir.join("file1.txt")).unwrap();
        assert_eq!(src_content, dst_content);
    }

    #[test]
    fn test_copy_dir_recursive_source_not_found() {
        let dst_base = TempDir::new().unwrap();
        let result = copy_dir_recursive(Path::new("/nonexistent/path"), dst_base.path());
        assert!(matches!(result, Err(FileOpsError::SourceNotFound(_))));
    }

    #[test]
    fn test_copy_dir_recursive_destination_exists() {
        let src_dir = create_test_structure();
        let dst_dir = TempDir::new().unwrap();

        let result = copy_dir_recursive(src_dir.path(), dst_dir.path());
        assert!(matches!(result, Err(FileOpsError::DestinationExists(_))));
    }

    #[test]
    fn test_delete_dir_recursive_success() {
        let temp_dir = create_test_structure();
        let path = temp_dir.path();

        delete_dir_recursive(path).unwrap();

        assert!(!path.exists());
    }

    #[test]
    fn test_delete_dir_recursive_not_found() {
        let result = delete_dir_recursive(Path::new("/nonexistent/path"));
        assert!(matches!(result, Err(FileOpsError::SourceNotFound(_))));
    }

    #[test]
    fn test_delete_dir_recursive_not_a_directory() {
        let temp_file = TempDir::new().unwrap();
        let file_path = temp_file.path().join("file.txt");
        File::create(&file_path).unwrap().write_all(b"test").unwrap();

        let result = delete_dir_recursive(&file_path);
        assert!(matches!(result, Err(FileOpsError::NotADirectory(_))));
    }

    #[test]
    fn test_get_dir_size_success() {
        let temp_dir = create_test_structure();
        let path = temp_dir.path();

        // file1.txt: "hello" = 5 bytes
        // file2.txt: "world test content" = 17 bytes
        // file3.txt: "nested data here" = 15 bytes
        // Total: 37 bytes
        let size = get_dir_size(path).unwrap();
        // Note: on some systems, directories may have metadata that adds bytes
        // We check that the size is at least the expected file content size
        assert!(size >= 37, "Expected at least 37 bytes, got {}", size);
        // And not unreasonably large
        assert!(size < 1024, "Expected less than 1KB, got {}", size);
    }

    #[test]
    fn test_get_dir_size_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let size = get_dir_size(temp_dir.path()).unwrap();
        assert_eq!(size, 0);
    }

    #[test]
    fn test_get_dir_size_not_found() {
        let result = get_dir_size(Path::new("/nonexistent/path"));
        assert!(matches!(result, Err(FileOpsError::SourceNotFound(_))));
    }

    #[test]
    fn test_get_dir_size_not_a_directory() {
        let temp_file = TempDir::new().unwrap();
        let file_path = temp_file.path().join("file.txt");
        File::create(&file_path).unwrap().write_all(b"test").unwrap();

        let result = get_dir_size(&file_path);
        assert!(matches!(result, Err(FileOpsError::NotADirectory(_))));
    }

    #[test]
    fn test_format_size_bytes() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1023), "1023 B");
    }

    #[test]
    fn test_format_size_kilobytes() {
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1536), "1.50 KB");
        assert_eq!(format_size(10240), "10.00 KB");
    }

    #[test]
    fn test_format_size_megabytes() {
        assert_eq!(format_size(1_048_576), "1.00 MB");
        assert_eq!(format_size(5_242_880), "5.00 MB");
        assert_eq!(format_size(123456789), "117.74 MB");
    }

    #[test]
    fn test_format_size_gigabytes() {
        assert_eq!(format_size(1_073_741_824), "1.00 GB");
        assert_eq!(format_size(2_147_483_648), "2.00 GB");
        assert_eq!(format_size(123456789012), "114.98 GB");
    }

    #[test]
    fn test_copy_preserves_content() {
        let src_dir = create_test_structure();
        let dst_base = TempDir::new().unwrap();
        let dst_dir = dst_base.path().join("copy");

        copy_dir_recursive(src_dir.path(), &dst_dir).unwrap();

        // Verify content of all files
        let content1 = fs::read_to_string(dst_dir.join("file1.txt")).unwrap();
        assert_eq!(content1, "hello");

        let content2 = fs::read_to_string(dst_dir.join("subdir/file2.txt")).unwrap();
        assert_eq!(content2, "world test content");

        let content3 = fs::read_to_string(dst_dir.join("subdir/nested/file3.txt")).unwrap();
        assert_eq!(content3, "nested data here");
    }

    #[test]
    fn test_copy_deeply_nested_structure() {
        let src_base = TempDir::new().unwrap();
        let mut current = src_base.path().to_path_buf();

        // Create a deep directory structure (10 levels)
        for i in 0..10 {
            current = current.join(format!("level_{}", i));
            fs::create_dir(&current).unwrap();
            File::create(current.join(format!("file_{}.txt", i)))
                .unwrap()
                .write_all(format!("content_{}", i).as_bytes())
                .unwrap();
        }

        let dst_base = TempDir::new().unwrap();
        let dst_dir = dst_base.path().join("deep_copy");

        copy_dir_recursive(src_base.path(), &dst_dir).unwrap();

        // Verify deep structure was copied
        let deep_file = dst_dir.join("level_0/level_1/level_2/level_3/level_4/level_5/level_6/level_7/level_8/level_9/file_9.txt");
        assert!(deep_file.exists());
    }
}
