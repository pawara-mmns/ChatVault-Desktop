use super::{Result, error};
use crate::storage::paths;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use tempfile::TempDir;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Owner {
    version: u32,
    id: String,
    installation: String,
}
pub const STAGE_MARKER: &str = ".chatvault-stage.json";
pub const FINAL_MARKER: &str = ".chatvault-import.json";

pub fn write_owner(root: &Path, marker: &str, id: &str, installation: &str) -> Result<()> {
    let bytes = serde_json::to_vec(&Owner {
        version: 1,
        id: id.into(),
        installation: installation.into(),
    })
    .map_err(|_| {
        error(
            "import_io",
            "Could not prepare the import ownership record.",
        )
    })?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(root.join(marker))
        .map_err(|_| {
            error(
                "import_io",
                "Could not create a new import directory without overwriting files.",
            )
        })?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| error("import_io", "Could not save the import ownership record."))
}
fn owner(root: &Path, marker: &str) -> Result<Owner> {
    paths::verify_ancestors(root, false)?;
    let file = paths::open_regular_file(&root.join(marker))?;
    let mut bytes = Vec::new();
    file.take(4097)
        .read_to_end(&mut bytes)
        .map_err(|_| error("import_io", "Could not validate import ownership."))?;
    if bytes.len() > 4096 {
        return Err(error(
            "import_recovery",
            "Import recovery found an invalid ownership record. Files were preserved.",
        ));
    }
    serde_json::from_slice(&bytes).map_err(|_| {
        error(
            "import_recovery",
            "Import recovery found an invalid ownership record. Files were preserved.",
        )
    })
}

/// Never remove a directory unless its identity and all ancestors/entries are validated.
pub fn verify_archive_owner(root: &Path, id: &str, installation: &str) -> Result<()> {
    let record = owner(root, FINAL_MARKER)?;
    if record.version != 1 || record.id != id || record.installation != installation {
        return Err(error(
            "unsafe_import",
            "Archive ownership could not be verified.",
        ));
    }
    Ok(())
}

pub fn cleanup_owned(root: &Path, marker: &str, id: &str, installation: &str) -> Result<()> {
    if !root.exists() {
        return Ok(());
    }
    let record = owner(root, marker)?;
    if record.version != 1 || record.id != id || record.installation != installation {
        return Err(error(
            "import_recovery",
            "Import ownership could not be verified. Files were preserved for manual inspection.",
        ));
    }
    let mut pending = vec![(root.to_path_buf(), 0)];
    let mut count = 0;
    while let Some((directory, depth)) = pending.pop() {
        if depth > 64 {
            return Err(error(
                "import_recovery",
                "Import cleanup exceeded its safety limits. Files were preserved.",
            ));
        }
        paths::verify_ancestors(&directory, false)?;
        for entry in fs::read_dir(directory)
            .map_err(|_| error("import_io", "Could not clean owned import staging."))?
        {
            let entry =
                entry.map_err(|_| error("import_io", "Could not inspect owned import staging."))?;
            count += 1;
            if count > 20_000 {
                return Err(error(
                    "import_recovery",
                    "Import cleanup exceeded its safety limits. Files were preserved.",
                ));
            }
            let metadata = fs::symlink_metadata(entry.path())
                .map_err(|_| error("import_io", "Could not inspect owned import staging."))?;
            if paths::is_link(&metadata) {
                return Err(error(
                    "import_recovery",
                    "Import cleanup found a link. Files were preserved.",
                ));
            }
            if metadata.is_dir() {
                pending.push((entry.path(), depth + 1));
            } else {
                paths::open_regular_file(&entry.path())?;
            }
        }
    }
    fs::remove_dir_all(root).map_err(|_| {
        error(
            "import_recovery",
            "Owned import staging could not be cleaned. It will be checked again on startup.",
        )
    })
}

pub struct Stage {
    directory: Option<TempDir>,
    lease: Option<File>,
    pub id: String,
    installation: String,
}
impl Stage {
    pub fn new(vault: &Path, installation: &str) -> Result<Self> {
        paths::verify_ancestors(&vault.join("temp"), false)?;
        let id = uuid::Uuid::new_v4().to_string();
        let directory = tempfile::Builder::new()
            .prefix(&format!("chatvault-import-{id}-"))
            .tempdir_in(vault.join("temp"))
            .map_err(|_| {
                error(
                    "import_io",
                    "Could not create private import staging inside the vault.",
                )
            })?;
        write_owner(directory.path(), STAGE_MARKER, &id, installation)?;
        let lease = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(directory.path().join("lease.lock"))
            .map_err(|_| error("import_io", "Could not lock import staging."))?;
        lease
            .try_lock()
            .map_err(|_| error("import_busy", "This import staging area is already in use."))?;
        fs::create_dir(directory.path().join("payload"))
            .map_err(|_| error("import_io", "Could not prepare import staging."))?;
        fs::create_dir(directory.path().join("payload/original"))
            .map_err(|_| error("import_io", "Could not prepare import staging."))?;
        fs::create_dir(directory.path().join("payload/media"))
            .map_err(|_| error("import_io", "Could not prepare import staging."))?;
        fs::create_dir(directory.path().join("work"))
            .map_err(|_| error("import_io", "Could not prepare import staging."))?;
        Ok(Self {
            directory: Some(directory),
            lease: Some(lease),
            id,
            installation: installation.into(),
        })
    }
    pub fn root(&self) -> &Path {
        self.directory.as_ref().expect("live stage").path()
    }
    pub fn payload(&self) -> PathBuf {
        self.root().join("payload")
    }
}
impl Drop for Stage {
    fn drop(&mut self) {
        self.lease.take();
        if let Some(directory) = self.directory.take() {
            // Disable TempDir's unconditional cleanup; our validator owns the deletion decision.
            let root = directory.keep();
            let _ = cleanup_owned(&root, STAGE_MARKER, &self.id, &self.installation);
        }
    }
}

pub fn recover_stages(vault: &Path, installation: &str) -> Result<()> {
    paths::verify_ancestors(&vault.join("temp"), false)?;
    let mut count = 0;
    for entry in fs::read_dir(vault.join("temp"))
        .map_err(|_| error("import_io", "Could not inspect import staging."))?
    {
        let entry = entry.map_err(|_| error("import_io", "Could not inspect import staging."))?;
        if !entry
            .file_name()
            .to_string_lossy()
            .starts_with("chatvault-import-")
        {
            continue;
        }
        count += 1;
        if count > 1000 {
            return Err(error(
                "import_recovery",
                "Too many abandoned import sessions need inspection.",
            ));
        }
        let record = match owner(&entry.path(), STAGE_MARKER) {
            Ok(value) if value.installation == installation && value.version == 1 => value,
            _ => continue,
        };
        let lease = match paths::open_regular_file(&entry.path().join("lease.lock")) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if lease.try_lock().is_err() {
            continue;
        } // Another application instance still owns it.
        drop(lease);
        cleanup_owned(&entry.path(), STAGE_MARKER, &record.id, installation)?;
    }
    Ok(())
}

pub fn copy_new_tree(source: &Path, destination: &Path) -> Result<()> {
    paths::verify_ancestors(source, false)?;
    for entry in fs::read_dir(source)
        .map_err(|_| error("import_io", "Could not read staged import files."))?
    {
        let entry = entry.map_err(|_| error("import_io", "Could not read staged import files."))?;
        let target = destination.join(entry.file_name());
        let metadata = fs::symlink_metadata(entry.path())
            .map_err(|_| error("import_io", "Could not inspect staged import files."))?;
        if paths::is_link(&metadata) {
            return Err(error(
                "unsafe_import",
                "A staged import path became a link. The import was stopped.",
            ));
        }
        if metadata.is_dir() {
            fs::create_dir(&target).map_err(|_| {
                error(
                    "import_io",
                    "Could not create collision-free archive storage.",
                )
            })?;
            copy_new_tree(&entry.path(), &target)?;
        } else {
            let mut input = paths::open_regular_file(&entry.path())?;
            let mut output = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(target)
                .map_err(|_| {
                    error(
                        "import_io",
                        "Could not store an import file without overwriting another file.",
                    )
                })?;
            std::io::copy(&mut input, &mut output)
                .and_then(|_| output.sync_all())
                .map_err(|_| error("import_io", "Could not persist the staged import files."))?;
        }
    }
    Ok(())
}
