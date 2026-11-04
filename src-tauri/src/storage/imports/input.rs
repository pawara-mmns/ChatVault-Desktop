use super::{Result, error, models::ImportLimits, staging::Stage};
use crate::storage::paths;
use sha2::{Digest, Sha256};
use std::{
    fs::OpenOptions,
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub struct Input {
    pub filename: String,
    pub size: u64,
    pub format: String,
    pub hash: String,
    pub stored: PathBuf,
}
pub fn copy_source(source: &Path, stage: &Stage, limits: &ImportLimits) -> Result<Input> {
    paths::verify_ancestors(
        source
            .parent()
            .ok_or_else(|| error("invalid_file", "Choose a local export file."))?,
        false,
    )?;
    let filename = source
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| value.chars().count() <= 512)
        .ok_or_else(|| error("invalid_file", "The selected filename is unsupported."))?
        .to_string();
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !matches!(extension.as_str(), "txt" | "zip") {
        return Err(error(
            "unsupported_format",
            "Choose a .txt or .zip chat export. Encrypted .crypt14/.crypt15 backups are not supported.",
        ));
    }
    let mut file = paths::open_regular_file(source)?;
    let before = file
        .metadata()
        .map_err(|_| error("invalid_file", "The source file cannot be inspected."))?;
    let limit = if extension == "txt" {
        limits.text_bytes
    } else {
        limits.source_bytes
    };
    if before.len() == 0 || before.len() > limit {
        return Err(error(
            "import_limit",
            "The selected export is empty or exceeds the configured file size limit.",
        ));
    }
    let stored = stage
        .payload()
        .join("original")
        .join(format!("source.{extension}"));
    let mut output = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&stored)
        .map_err(|_| error("import_io", "Could not stage the selected export."))?;
    let mut hash = Sha256::new();
    let mut size = 0u64;
    let mut buffer = [0u8; 65536];
    let mut prefix = Vec::new();
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|_| error("invalid_file", "The source file could not be read."))?;
        if read == 0 {
            break;
        }
        size += read as u64;
        if size > limit {
            return Err(error(
                "import_limit",
                "The source grew beyond the import size limit.",
            ));
        }
        if prefix.len() < 4 {
            prefix.extend(buffer[..read].iter().take(4 - prefix.len()));
        }
        hash.update(&buffer[..read]);
        output
            .write_all(&buffer[..read])
            .map_err(|_| error("import_io", "Could not stage the selected export."))?;
    }
    output
        .sync_all()
        .map_err(|_| error("import_io", "Could not sync the staged export."))?;
    let after = file
        .metadata()
        .map_err(|_| error("invalid_file", "The source file cannot be inspected."))?;
    if size != before.len()
        || after.len() != before.len()
        || after.modified().ok() != before.modified().ok()
    {
        return Err(error(
            "source_changed",
            "The selected export changed while it was being read. Choose a stable copy and retry.",
        ));
    }
    let zip_magic = prefix.starts_with(b"PK\x03\x04") || prefix.starts_with(b"PK\x05\x06");
    if (extension == "zip") != zip_magic {
        return Err(error(
            "unsupported_format",
            "The file contents do not match the selected text/ZIP format.",
        ));
    }
    Ok(Input {
        filename,
        size,
        format: extension,
        hash: format!("{:x}", hash.finalize()),
        stored,
    })
}

pub fn read_text(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let file = paths::open_regular_file(path)?;
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| error("invalid_file", "The staged text cannot be read."))?;
    if bytes.len() as u64 > limit {
        return Err(error(
            "import_limit",
            "The conversation text exceeds the configured size limit.",
        ));
    }
    Ok(bytes)
}
