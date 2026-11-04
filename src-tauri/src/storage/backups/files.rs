use super::models::*;
use crate::storage::paths;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
pub const MARKER: &str = ".chatvault-backup.json";
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Owner {
    version: u32,
    snapshot: String,
    installation: String,
}
pub fn safe_component(name: &str) -> String {
    let mut safe: String = name
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') {
                '_'
            } else {
                c
            }
        })
        .take(60)
        .collect();
    safe = safe.trim_matches([' ', '.']).to_owned();
    let stem = safe.split('.').next().unwrap_or("").to_ascii_uppercase();
    if safe.is_empty()
        || matches!(
            stem.as_str(),
            "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
        )
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && matches!(stem.as_bytes()[3], b'1'..=b'9'))
    {
        safe.insert(0, '_');
    }
    safe
}
pub fn join(root: &Path, relative: &str) -> Result<PathBuf> {
    if relative.is_empty()
        || relative.len() > 2400
        || relative.contains(['\\', ':'])
        || relative.split('/').any(|s| {
            s.is_empty()
                || s == "."
                || s == ".."
                || s.chars().count() > 120
                || s.ends_with(['.', ' '])
                || s.chars().any(|c| {
                    c.is_control()
                        || matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')
                })
                || {
                    let stem = s.split('.').next().unwrap_or("").to_ascii_uppercase();
                    matches!(
                        stem.as_str(),
                        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
                    ) || stem.len() == 4
                        && (stem.starts_with("COM") || stem.starts_with("LPT"))
                        && matches!(stem.as_bytes()[3], b'1'..=b'9')
                }
        })
    {
        return Err(fail("unsafe_path"));
    }
    let path = root.join(relative);
    paths::validate_shape(&path)?;
    paths::verify_ancestors(path.parent().ok_or_else(|| fail("unsafe_path"))?, true)?;
    Ok(path)
}
pub fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    paths::verify_ancestors(path.parent().ok_or_else(|| fail("unsafe_path"))?, false)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(io)?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(io)
}
pub fn owner(root: &Path, id: &str, installation: &str) -> Result<()> {
    let bytes = serde_json::to_vec(&Owner {
        version: 1,
        snapshot: id.into(),
        installation: installation.into(),
    })
    .map_err(|_| fail("failed"))?;
    write_new(&root.join(MARKER), &bytes)
}
pub fn verify_owner(root: &Path, id: &str, installation: &str) -> Result<()> {
    paths::verify_ancestors(root, false)?;
    let mut bytes = vec![];
    paths::open_regular_file(&root.join(MARKER))?
        .take(4097)
        .read_to_end(&mut bytes)
        .map_err(io)?;
    let o: Owner = serde_json::from_slice(&bytes).map_err(|_| fail("unsafe_path"))?;
    if bytes.len() > 4096 || o.version != 1 || o.snapshot != id || o.installation != installation {
        return Err(fail("unsafe_path"));
    }
    Ok(())
}
pub fn rename_new(source: &Path, target: &Path) -> Result<()> {
    paths::verify_ancestors(source.parent().ok_or_else(|| fail("unsafe_path"))?, false)?;
    paths::verify_ancestors(target.parent().ok_or_else(|| fail("unsafe_path"))?, false)?;
    if fs::symlink_metadata(target).is_ok() {
        return Err(fail("unsafe_path"));
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{MOVEFILE_WRITE_THROUGH, MoveFileExW};
        let a: Vec<_> = source.as_os_str().encode_wide().chain(Some(0)).collect();
        let b: Vec<_> = target.as_os_str().encode_wide().chain(Some(0)).collect();
        if unsafe { MoveFileExW(a.as_ptr(), b.as_ptr(), MOVEFILE_WRITE_THROUGH) } == 0 {
            return Err(io(std::io::Error::last_os_error()));
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        fs::rename(source, target).map_err(io)
    }
}
pub fn free_bytes(root: &Path) -> Option<u64> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
        let root: Vec<_> = root.as_os_str().encode_wide().chain(Some(0)).collect();
        let mut free = 0;
        let ok = unsafe {
            GetDiskFreeSpaceExW(
                root.as_ptr(),
                &mut free,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        (ok != 0).then_some(free)
    }
    #[cfg(not(windows))]
    {
        let _ = root;
        None
    }
}
pub fn validate_tree(root: &Path) -> Result<()> {
    let mut pending = vec![(root.to_path_buf(), 0)];
    let mut count = 0;
    while let Some((dir, depth)) = pending.pop() {
        if depth > 24 {
            return Err(fail("limit"));
        }
        paths::verify_ancestors(&dir, false)?;
        for e in fs::read_dir(dir).map_err(io)? {
            let e = e.map_err(io)?;
            count += 1;
            if count > 100_100 {
                return Err(fail("limit"));
            }
            let m = fs::symlink_metadata(e.path()).map_err(io)?;
            if paths::is_link(&m) {
                return Err(fail("unsafe_path"));
            }
            if m.is_dir() {
                pending.push((e.path(), depth + 1));
            } else {
                paths::open_regular_file(&e.path())?;
            }
        }
    }
    Ok(())
}
pub fn cleanup(root: &Path, id: &str, installation: &str) -> Result<()> {
    if !root.exists() {
        return Ok(());
    }
    verify_owner(root, id, installation)?;
    validate_tree(root)?;
    fs::remove_dir_all(root).map_err(io)
}
pub fn manifest(root: &Path, manifest: &Manifest) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(manifest).map_err(|_| fail("failed"))?;
    if bytes.len() > 64 * 1024 * 1024 {
        return Err(fail("limit"));
    }
    write_new(&root.join("manifest.json.partial"), &bytes)?;
    rename_new(
        &root.join("manifest.json.partial"),
        &root.join("manifest.json"),
    )
}
pub fn read_manifest(root: &Path) -> Result<Manifest> {
    let file = paths::open_regular_file(&root.join("manifest.json"))?;
    if file.metadata().map_err(io)?.len() > 64 * 1024 * 1024 {
        return Err(fail("limit"));
    }
    serde_json::from_reader(file).map_err(|_| fail("unsafe_path"))
}
pub fn hash_local(
    path: &Path,
    budget: &crate::devices::adapter::Budget,
    mut progress: impl FnMut(u64),
) -> Result<(u64, String)> {
    use sha2::{Digest, Sha256};
    let mut file = paths::open_regular_file(path)?;
    let mut hash = Sha256::new();
    let mut buffer = vec![0; 128 * 1024];
    let mut bytes = 0;
    loop {
        budget.check()?;
        let n = file.read(&mut buffer).map_err(io)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
        bytes += n as u64;
        if bytes > super::planner::MAX_FILE_BYTES {
            return Err(fail("limit"));
        }
        progress(bytes);
    }
    Ok((bytes, format!("{:x}", hash.finalize())))
}
pub fn create_file(path: &Path) -> Result<File> {
    paths::verify_ancestors(path.parent().ok_or_else(|| fail("unsafe_path"))?, false)?;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(io)
}
