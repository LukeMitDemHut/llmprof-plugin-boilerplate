//! Storage helpers using the native WASI filesystem.
//!
//! The LLMProf Extism host mounts a persistent storage directory at `/storage`.
//! Rust plugins can use the standard library `std::fs` module directly — no
//! host functions are required for file I/O. This is one of the key advantages
//! of Rust on WASI: full filesystem access via the standard library.
//!
//! # Path safety
//!
//! All public functions in this module reject absolute paths and `..`
//! traversal segments, ensuring that plugin code cannot escape the `/storage`
//! root.

use std::fs;
use std::path::{Path, PathBuf};

/// The root directory of the plugin's persistent storage, mounted by the host.
const STORAGE_ROOT: &str = "/storage";

/// Validate that a user-supplied filename is safe to use under `/storage`.
///
/// Rejects:
/// - Absolute paths (starting with `/`)
/// - Paths containing `..` segments (directory traversal)
///
/// Returns the joined, safe path relative to `STORAGE_ROOT`.
fn safe_path(filename: &str) -> Result<PathBuf, String> {
    // Reject absolute paths.
    if filename.starts_with('/') {
        return Err(format!(
            "Absolute paths are not allowed: '{filename}'"
        ));
    }

    let path = Path::new(filename);

    // Check each component for `..` traversal.
    for component in path.components() {
        let comp_str = component.as_os_str().to_string_lossy();
        if comp_str == ".." {
            return Err(format!(
                "Path traversal ('..') is not allowed: '{filename}'"
            ));
        }
    }

    // Join with the storage root.
    let full_path = Path::new(STORAGE_ROOT).join(path);
    Ok(full_path)
}

/// Read a file from the plugin's storage directory.
///
/// # Arguments
///
/// * `filename` — Relative path within `/storage` (e.g. `"data.json"` or
///   `"subdir/file.txt"`).
///
/// # Returns
///
/// The file contents as a `String`, or an error message if the file does not
/// exist, cannot be read, or contains invalid UTF-8.
pub fn read_file(filename: &str) -> Result<String, String> {
    let path = safe_path(filename)?;

    fs::read_to_string(&path).map_err(|e| {
        format!("Failed to read '{}': {e}", path.display())
    })
}

/// Write a file to the plugin's storage directory.
///
/// Parent directories are created automatically if they do not exist.
///
/// # Arguments
///
/// * `filename` — Relative path within `/storage`.
/// * `content` — The string content to write.
///
/// # Returns
///
/// `Ok(())` on success, or an error message on failure.
pub fn write_file(filename: &str, content: &str) -> Result<(), String> {
    let path = safe_path(filename)?;

    // Create parent directories if needed.
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "Failed to create parent directories for '{}': {e}",
                path.display()
            )
        })?;
    }

    fs::write(&path, content).map_err(|e| {
        format!("Failed to write '{}': {e}", path.display())
    })
}

/// Recursively list all files under a path within the plugin's storage
/// directory.
///
/// # Arguments
///
/// * `path` — Relative path within `/storage` to list. Use `""` to list
///   everything in the storage root.
///
/// # Returns
///
/// A vector of file paths (relative to `/storage`), or an error message.
/// Directories are not included in the result; only files.
pub fn list_files(path: &str) -> Result<Vec<String>, String> {
    let base = safe_path(path)?;

    // If the path doesn't exist, return an empty list rather than an error.
    if !base.exists() {
        return Ok(Vec::new());
    }

    let mut results = Vec::new();

    // Walk the directory tree recursively.
    fn walk(
        current: &Path,
        root: &Path,
        results: &mut Vec<String>,
    ) -> Result<(), String> {
        let entries = fs::read_dir(current).map_err(|e| {
            format!("Failed to read directory '{}': {e}", current.display())
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                format!("Failed to read directory entry: {e}")
            })?;

            let entry_path = entry.path();

            if entry_path.is_dir() {
                // Recurse into subdirectories.
                walk(&entry_path, root, results)?;
            } else {
                // Compute the path relative to the storage root.
                let relative = entry_path
                    .strip_prefix(root)
                    .map_err(|e| {
                        format!("Failed to compute relative path: {e}")
                    })?
                    .to_string_lossy()
                    .to_string();

                results.push(relative);
            }
        }

        Ok(())
    }

    walk(&base, Path::new(STORAGE_ROOT), &mut results)?;

    Ok(results)
}

/// Delete a file from the plugin's storage directory.
///
/// # Arguments
///
/// * `filename` — Relative path within `/storage`.
///
/// # Returns
///
/// `Ok(())` on success, or an error message if the file does not exist or
/// cannot be deleted.
pub fn delete_file(filename: &str) -> Result<(), String> {
    let path = safe_path(filename)?;

    fs::remove_file(&path).map_err(|e| {
        format!("Failed to delete '{}': {e}", path.display())
    })
}