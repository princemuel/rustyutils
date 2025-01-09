use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Resolves the given file path by expanding user directories and canonicalizing relative paths.
///
/// # Arguments
/// - `path`: A reference to the path to resolve.
///
/// # Returns
/// - `Ok(PathBuf)`: The resolved absolute path.
/// - `Err(io::Error)`: If the path cannot be resolved.
pub fn resolve_path<P: AsRef<Path>>(path: P) -> io::Result<PathBuf> {
    // Expand `~` to home directory (e.g., `~` to `/home/user`).
    let path_str = path.as_ref().to_str().unwrap_or_default();
    let expanded = shellexpand::tilde(path_str).into_owned();

    fs::canonicalize(expanded)
}
