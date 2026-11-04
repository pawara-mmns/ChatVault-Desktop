use super::error::{Result, StorageError};
use std::{
    fs,
    path::{Component, Path, PathBuf},
};

pub fn is_link(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        // Reject every reparse point, including junctions, mount points, and cloud placeholders.
        if metadata.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    false
}

/// Reject hard links as well as reparse points, so managed files cannot alias outside files.
pub fn open_regular_file(path: &Path) -> Result<fs::File> {
    let metadata = fs::symlink_metadata(path).map_err(|_| StorageError::filesystem())?;
    if is_link(&metadata) || !metadata.is_file() {
        return Err(StorageError::unsafe_path());
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;
        options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    let file = options.open(path).map_err(|_| StorageError::filesystem())?;
    let metadata = file.metadata().map_err(|_| StorageError::filesystem())?;
    if is_link(&metadata) || !metadata.is_file() {
        return Err(StorageError::unsafe_path());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.nlink() != 1 {
            return Err(StorageError::unsafe_path());
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Storage::FileSystem::{
            BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
        };
        // SAFETY: this POD struct is filled through a valid pointer and a live file handle.
        let mut information: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
        let succeeded =
            unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut information) };
        if succeeded == 0 {
            return Err(StorageError::filesystem());
        }
        if information.nNumberOfLinks != 1 || information.dwFileAttributes & 0x400 != 0 {
            return Err(StorageError::unsafe_path());
        }
    }
    Ok(file)
}

pub fn validate_shape(path: &Path) -> Result<()> {
    let text = path.to_str().ok_or_else(StorageError::unsafe_path)?;
    if !path.is_absolute()
        || text.chars().any(char::is_control)
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
    {
        return Err(StorageError::unsafe_path());
    }
    #[cfg(windows)]
    {
        use std::path::Prefix;
        use windows_sys::Win32::Storage::FileSystem::GetDriveTypeW;
        let drive = match path.components().next() {
            Some(Component::Prefix(prefix)) => match prefix.kind() {
                Prefix::Disk(drive) => drive,
                _ => return Err(StorageError::unsafe_path()),
            },
            _ => return Err(StorageError::unsafe_path()),
        };
        let root = [u16::from(drive), u16::from(b':'), u16::from(b'\\'), 0];
        // Fixed and removable local volumes only. Reject disconnected, remote, optical, and RAM drives.
        // SAFETY: root is a live, null-terminated UTF-16 drive-root string.
        let kind = unsafe { GetDriveTypeW(root.as_ptr()) };
        if kind != 2 && kind != 3 {
            return Err(StorageError::unsafe_path());
        }
        for part in path.components() {
            if let Component::Normal(name) = part {
                let name = name.to_str().ok_or_else(StorageError::unsafe_path)?;
                let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
                if name.ends_with(['.', ' '])
                    || name.contains(['<', '>', ':', '"', '|', '?', '*'])
                    || matches!(
                        stem.as_str(),
                        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
                    )
                    || (stem.len() == 4
                        && (stem.starts_with("COM") || stem.starts_with("LPT"))
                        && matches!(stem.as_bytes()[3], b'1'..=b'9'))
                {
                    return Err(StorageError::unsafe_path());
                }
            }
        }
    }
    Ok(())
}

pub fn verify_ancestors(path: &Path, allow_missing: bool) -> Result<()> {
    validate_shape(path)?;
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) if is_link(&metadata) || !metadata.is_dir() => {
                return Err(StorageError::unsafe_path());
            }
            Ok(_) => {}
            Err(error) if allow_missing && error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(StorageError::filesystem()),
        }
    }
    Ok(())
}

/// Only used for explicit initialization/selection, never while restoring a missing saved vault.
pub fn create_safe_directory(path: &Path) -> Result<()> {
    verify_ancestors(path, true)?;
    let mut missing: Vec<PathBuf> = path
        .ancestors()
        .take_while(|part| !part.exists())
        .map(Path::to_path_buf)
        .collect();
    missing.reverse();
    for directory in missing {
        let parent = directory.parent().ok_or_else(StorageError::unsafe_path)?;
        verify_ancestors(parent, false)?;
        match fs::create_dir(&directory) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(_) => return Err(StorageError::filesystem()),
        }
        verify_ancestors(&directory, false)?;
    }
    verify_ancestors(path, false)
}

pub fn canonical(path: &Path) -> Result<PathBuf> {
    verify_ancestors(path, false)?;
    let result = fs::canonicalize(path).map_err(|_| StorageError::filesystem())?;
    #[cfg(windows)]
    let result = {
        let text = result.to_str().ok_or_else(StorageError::unsafe_path)?;
        PathBuf::from(text.strip_prefix(r"\\?\").unwrap_or(text))
    };
    validate_shape(&result)?;
    Ok(result)
}

fn contains(parent: &Path, child: &Path) -> bool {
    #[cfg(windows)]
    {
        let parent = parent.to_string_lossy().to_lowercase();
        let child = child.to_string_lossy().to_lowercase();
        child == parent || child.starts_with(&(parent.trim_end_matches('\\').to_owned() + "\\"))
    }
    #[cfg(not(windows))]
    {
        child.starts_with(parent)
    }
}

pub fn validate_vault_location(path: &Path, protected: &[PathBuf]) -> Result<()> {
    validate_shape(path)?;
    verify_ancestors(path, true)?;
    // Resolve existing ancestors before any creation, including Windows short-name aliases.
    let resolve_candidate = |candidate: &Path| -> Result<PathBuf> {
        let mut missing = Vec::new();
        let mut existing = candidate;
        while !existing.exists() {
            missing.push(
                existing
                    .file_name()
                    .ok_or_else(StorageError::unsafe_path)?
                    .to_os_string(),
            );
            existing = existing.parent().ok_or_else(StorageError::unsafe_path)?;
        }
        let mut resolved = canonical(existing)?;
        for name in missing.into_iter().rev() {
            resolved.push(name);
        }
        Ok(resolved)
    };
    let normalized = resolve_candidate(path)?;
    if normalized.parent().is_none()
        || protected.iter().any(|other| {
            let other = resolve_candidate(other).unwrap_or_else(|_| other.clone());
            contains(&other, &normalized) || contains(&normalized, &other)
        })
    {
        return Err(StorageError::unsafe_path());
    }
    Ok(())
}

pub fn write_probe(path: &Path) -> Result<()> {
    verify_ancestors(path, false)?;
    if fs::metadata(path)
        .map_err(|_| StorageError::filesystem())?
        .permissions()
        .readonly()
    {
        return Err(StorageError::new(
            "read_only",
            "The vault folder is read-only. Choose a writable local directory or correct its permissions.",
        ));
    }
    // tempfile uses exclusive creation and random names. It removes only its own probe on drop.
    let mut probe = tempfile::Builder::new().prefix(".chatvault-write-check-").tempfile_in(path).map_err(|_| StorageError::new("read_only", "The vault cannot be written to. Check folder permissions and available disk space."))?;
    use std::io::Write;
    probe
        .write_all(b"ChatVault write check")
        .and_then(|_| probe.as_file().sync_all())
        .map_err(|_| StorageError::filesystem())?;
    probe.close().map_err(|_| StorageError::filesystem())
}
