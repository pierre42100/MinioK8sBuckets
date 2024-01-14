use std::path::{Path, PathBuf};

/// Get the directory where temp files should be created
fn temp_path() -> Option<PathBuf> {
    std::env::var("TEMP_DIR")
        .as_deref()
        .ok()
        .map(Path::new)
        .map(|p| p.to_path_buf())
}

/// Create a temporary directory
pub fn create_temp_dir() -> std::io::Result<mktemp::Temp> {
    match temp_path() {
        None => mktemp::Temp::new_dir(),
        Some(p) => mktemp::Temp::new_dir_in(p),
    }
}

/// Create a temporary file
pub fn create_temp_file() -> std::io::Result<mktemp::Temp> {
    match temp_path() {
        None => mktemp::Temp::new_file(),
        Some(p) => mktemp::Temp::new_file_in(p),
    }
}
