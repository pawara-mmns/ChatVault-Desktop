use super::{
    error::{Result, StorageError},
    paths, usage,
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub const DIRECTORIES: [&str; 4] = ["backups", "archives", "media", "temp"];
pub const MARKER: &str = ".chatvault-vault.json";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Marker {
    format_version: u32,
    installation_id: String,
}

pub(super) fn verify_marker(root: &Path, installation_id: &str) -> Result<()> {
    let path = root.join(MARKER);
    let metadata = fs::symlink_metadata(&path).map_err(|_| StorageError::new("unrecognized_vault", "This folder is not an initialized vault for this installation. Existing contents have not been adopted."))?;
    if paths::is_link(&metadata) || !metadata.is_file() || metadata.len() > 4096 {
        return Err(StorageError::unsafe_path());
    }
    let mut contents = String::new();
    paths::open_regular_file(&path)?
        .take(4097)
        .read_to_string(&mut contents)
        .map_err(|_| StorageError::filesystem())?;
    let marker: Marker = serde_json::from_str(&contents).map_err(|_| {
        StorageError::new(
            "unrecognized_vault",
            "The vault identity file is invalid. It has not been replaced.",
        )
    })?;
    if marker.format_version != 1 || marker.installation_id != installation_id {
        return Err(StorageError::new(
            "foreign_vault",
            "This vault belongs to another installation or format version. Adopting existing vault data is not supported.",
        ));
    }
    Ok(())
}

pub fn inspect(root: &Path, installation_id: &str, protected: &[PathBuf]) -> Result<usage::Usage> {
    if fs::symlink_metadata(root).is_err() {
        return Err(StorageError::filesystem());
    }
    paths::validate_vault_location(root, protected)?;
    paths::verify_ancestors(root, false)?;
    verify_marker(root, installation_id)?;
    for name in DIRECTORIES {
        paths::verify_ancestors(&root.join(name), false)?;
        paths::write_probe(&root.join(name))?;
    }
    paths::write_probe(root)?;
    usage::calculate(root)
}

/// Initialize only a new/empty folder or an empty, validated vault of this installation.
/// Existing data is never adopted by a location change.
pub fn prepare(
    root: &Path,
    installation_id: &str,
    protected: &[PathBuf],
    existing_current: bool,
) -> Result<PathBuf> {
    paths::validate_vault_location(root, protected)?;
    paths::create_safe_directory(root)?;
    let root = paths::canonical(root)?;
    paths::validate_vault_location(&root, protected)?;
    let marker_exists = match fs::symlink_metadata(root.join(MARKER)) {
        Ok(_) => true,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(_) => return Err(StorageError::filesystem()),
    };
    if marker_exists {
        verify_marker(&root, installation_id)?;
        // Unknown files or nested entries make a candidate nonempty, even if zero bytes.
        if !existing_current && usage::calculate(&root)?.has_data {
            return Err(StorageError::new(
                "destination_not_empty",
                "Choose an empty folder. Existing ChatVault data cannot be adopted or replaced in this step.",
            ));
        }
    } else {
        if fs::read_dir(&root)
            .map_err(|_| StorageError::filesystem())?
            .next()
            .is_some()
        {
            return Err(StorageError::new(
                "destination_not_empty",
                "Choose an empty folder. Existing files have not been changed or adopted.",
            ));
        }
        paths::write_probe(&root)?;
        let bytes = serde_json::to_vec(&Marker {
            format_version: 1,
            installation_id: installation_id.to_owned(),
        })
        .map_err(|_| StorageError::filesystem())?;
        // Exclusive creation: no user file or prior marker can be overwritten.
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(root.join(MARKER))
            .map_err(|_| StorageError::filesystem())?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| StorageError::filesystem())?;
    }
    for name in DIRECTORIES {
        paths::create_safe_directory(&root.join(name))?;
    }
    inspect(&root, installation_id, protected)?;
    Ok(root)
}
