use super::{
    error::{Result, StorageError},
    paths,
    vault::{DIRECTORIES, MARKER},
};
use std::{fs, path::Path};

#[derive(Debug)]
pub struct Usage {
    pub bytes: u64,
    pub has_data: bool,
}

/// Logical file sizes, not allocated disk blocks. Never follow links or reparse points.
pub fn calculate(root: &Path) -> Result<Usage> {
    paths::verify_ancestors(root, false)?;
    let mut pending = vec![(root.to_path_buf(), 0usize)];
    let mut entries = 0usize;
    let mut usage = Usage {
        bytes: 0,
        has_data: false,
    };
    while let Some((directory, depth)) = pending.pop() {
        if depth > 64 || entries > 100_000 {
            return Err(StorageError::new(
                "scan_limit",
                "The vault is too large or deeply nested for a safe usage scan in this version.",
            ));
        }
        paths::verify_ancestors(&directory, false)?;
        for entry in fs::read_dir(&directory).map_err(|_| StorageError::filesystem())? {
            let entry = entry.map_err(|_| StorageError::filesystem())?;
            entries += 1;
            if entries > 100_000 {
                return Err(StorageError::new(
                    "scan_limit",
                    "The vault contains too many entries for a safe usage scan in this version.",
                ));
            }
            let metadata =
                fs::symlink_metadata(entry.path()).map_err(|_| StorageError::filesystem())?;
            if paths::is_link(&metadata) {
                return Err(StorageError::unsafe_path());
            }
            let at_root = depth == 0;
            if at_root && entry.file_name() == MARKER {
                if !metadata.is_file() {
                    return Err(StorageError::unsafe_path());
                }
                continue;
            }
            let managed_empty_directory = at_root
                && metadata.is_dir()
                && DIRECTORIES.iter().any(|name| entry.file_name() == *name);
            if !managed_empty_directory {
                usage.has_data = true;
            }
            if metadata.is_file() {
                let actual = paths::open_regular_file(&entry.path())?
                    .metadata()
                    .map_err(|_| StorageError::filesystem())?;
                usage.bytes = usage
                    .bytes
                    .checked_add(actual.len())
                    .ok_or_else(StorageError::filesystem)?;
            } else if metadata.is_dir() {
                pending.push((entry.path(), depth + 1));
            } else {
                return Err(StorageError::unsafe_path());
            }
        }
    }
    Ok(usage)
}
