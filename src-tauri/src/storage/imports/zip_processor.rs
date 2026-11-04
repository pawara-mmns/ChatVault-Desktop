use super::{
    Result, error,
    models::{Diagnostic, ImportLimits, diagnostic},
    staging::Stage,
};
use crate::storage::paths;
use std::{
    collections::HashSet,
    fs::OpenOptions,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};
use unicode_normalization::UnicodeNormalization;
use zip::{CompressionMethod, ZipArchive};

#[derive(Clone)]
pub struct MediaFile {
    pub original_name: String,
    pub stored_name: String,
    pub size: u64,
    pub media_type: String,
}
pub struct Extracted {
    pub texts: Vec<(String, PathBuf)>,
    pub media: Vec<MediaFile>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn safe_name(name: &str) -> Result<String> {
    if name.len() > 512
        || name.starts_with('/')
        || name.contains('\\')
        || name.chars().any(char::is_control)
    {
        return Err(error(
            "unsafe_zip",
            "The ZIP contains an unsafe filename or path.",
        ));
    }
    let name = name.trim_end_matches('/');
    if name.is_empty() {
        return Err(error("unsafe_zip", "The ZIP contains an empty filename."));
    }
    for component in name.split('/') {
        let stem = component
            .split('.')
            .next()
            .unwrap_or("")
            .to_ascii_uppercase();
        if component.is_empty()
            || matches!(component, "." | "..")
            || component.ends_with(['.', ' '])
            || component.contains(['<', '>', ':', '"', '|', '?', '*'])
            || matches!(
                stem.as_str(),
                "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
            )
            || (stem.len() == 4
                && (stem.starts_with("COM") || stem.starts_with("LPT"))
                && matches!(stem.as_bytes()[3], b'1'..=b'9'))
        {
            return Err(error(
                "unsafe_zip",
                "The ZIP contains a reserved, absolute, or traversal filename.",
            ));
        }
    }
    Ok(name.nfc().collect::<String>().to_lowercase())
}

pub fn media_kind(name: &str) -> Option<(&'static str, String)> {
    let extension = name.rsplit('.').next()?.to_ascii_lowercase();
    let kind = match extension.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "heic" | "heif" => "image/heic",
        "mp4" | "3gp" => "video/mp4",
        "mov" => "video/quicktime",
        "mp3" => "audio/mpeg",
        "opus" | "ogg" => "audio/ogg",
        "m4a" | "aac" => "audio/mp4",
        "wav" => "audio/wav",
        "pdf" => "application/pdf",
        "vcf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" => "application/octet-stream",
        _ => return None,
    };
    Some((kind, extension))
}

fn preflight(file: &mut std::fs::File, limits: &ImportLimits) -> Result<usize> {
    let length = file
        .metadata()
        .map_err(|_| error("invalid_zip", "Could not inspect the ZIP."))?
        .len();
    let size = length.min(65557);
    file.seek(SeekFrom::End(-(size as i64)))
        .map_err(|_| error("invalid_zip", "Could not inspect the ZIP footer."))?;
    let mut tail = vec![0; size as usize];
    file.read_exact(&mut tail)
        .map_err(|_| error("invalid_zip", "The ZIP footer is truncated."))?;
    let at = (0..tail.len().saturating_sub(21))
        .rev()
        .find(|&i| {
            tail[i..].starts_with(b"PK\x05\x06")
                && i + 22 + u16::from_le_bytes([tail[i + 20], tail[i + 21]]) as usize == tail.len()
        })
        .ok_or_else(|| {
            error(
                "invalid_zip",
                "The ZIP is truncated, corrupted, or uses an unsupported ZIP format.",
            )
        })?;
    let u16at = |i| u16::from_le_bytes([tail[at + i], tail[at + i + 1]]);
    let u32at = |i| u32::from_le_bytes(tail[at + i..at + i + 4].try_into().unwrap());
    let entries = u16at(10) as usize;
    if u16at(4) != 0
        || u16at(6) != 0
        || u16at(8) as usize != entries
        || entries == 65535
        || u32at(12) == u32::MAX
        || u32at(16) == u32::MAX
    {
        return Err(error(
            "unsupported_zip",
            "Split and ZIP64 archives are not supported by this importer.",
        ));
    }
    if entries == 0 || entries > limits.entries || u32at(12) > 16 * 1024 * 1024 {
        return Err(error(
            "import_limit",
            "The ZIP has no entries or exceeds its configured entry/metadata limits.",
        ));
    }
    Ok(entries)
}

pub fn extract(source: &Path, stage: &Stage, limits: &ImportLimits) -> Result<Extracted> {
    let mut input = paths::open_regular_file(source)?;
    let count = preflight(&mut input, limits)?;
    let mut zip = ZipArchive::new(input).map_err(|_| {
        error(
            "invalid_zip",
            "The ZIP cannot be opened. It may be corrupted or encrypted.",
        )
    })?;
    if zip.len() != count {
        return Err(error("invalid_zip", "The ZIP entry index is inconsistent."));
    }
    let mut result = Extracted {
        texts: Vec::new(),
        media: Vec::new(),
        diagnostics: Vec::new(),
    };
    let mut names = HashSet::new();
    let mut total = 0u64;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).map_err(|_| {
            error(
                "invalid_zip",
                "A ZIP entry cannot be read. Encrypted archives are unsupported.",
            )
        })?;
        let name = std::str::from_utf8(entry.name_raw())
            .map_err(|_| error("invalid_encoding", "ZIP filenames must use UTF-8 or ASCII."))?
            .to_string();
        if !names.insert(safe_name(&name)?) {
            return Err(error(
                "unsafe_zip",
                "The ZIP contains colliding filenames. Nothing was overwritten.",
            ));
        }
        let mode = entry.unix_mode().unwrap_or(0) & 0o170000;
        if (entry.is_dir() && !matches!(mode, 0 | 0o040000))
            || (!entry.is_dir() && !matches!(mode, 0 | 0o100000))
        {
            return Err(error(
                "unsafe_zip",
                "ZIP links and special files are not supported.",
            ));
        }
        if !matches!(
            entry.compression(),
            CompressionMethod::Stored | CompressionMethod::Deflated
        ) {
            return Err(error(
                "unsupported_zip",
                "Only unencrypted Stored/Deflate ZIP entries are supported.",
            ));
        }
        let size = entry.size();
        total = total
            .checked_add(size)
            .ok_or_else(|| error("import_limit", "The ZIP expansion size is invalid."))?;
        if size > limits.entry_bytes
            || total > limits.expanded_bytes
            || size
                > entry
                    .compressed_size()
                    .max(1)
                    .saturating_mul(limits.compression_ratio)
        {
            return Err(error(
                "import_limit",
                "The ZIP exceeds expansion, per-file, or compression-ratio limits.",
            ));
        }
        if entry.is_dir() {
            continue;
        }
        let text = name.to_ascii_lowercase().ends_with(".txt");
        let kind = media_kind(&name);
        let target = if text {
            if result.texts.len() >= 32 || size > limits.text_bytes {
                return Err(error(
                    "import_limit",
                    "The ZIP contains too many or oversized text files.",
                ));
            }
            stage
                .root()
                .join("work")
                .join(format!("{}.txt", uuid::Uuid::new_v4()))
        } else if let Some((_, extension)) = &kind {
            stage
                .payload()
                .join("media")
                .join(format!("{}.{extension}", uuid::Uuid::new_v4()))
        } else {
            diagnostic(
                &mut result.diagnostics,
                "unsupported_media",
                "Some ZIP files use unsupported/active file types and were not extracted. The original ZIP is retained.",
                None,
            );
            // Still consume every entry with a hard bound so CRC/decompression errors cannot hide.
            let actual = std::io::copy(&mut entry.by_ref().take(size + 1), &mut std::io::sink())
                .map_err(|_| {
                    error(
                        "invalid_zip",
                        "A ZIP entry failed decompression or integrity validation.",
                    )
                })?;
            if actual != size {
                return Err(error(
                    "invalid_zip",
                    "A ZIP entry size does not match its contents.",
                ));
            }
            continue;
        };
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
            .map_err(|_| {
                error(
                    "import_io",
                    "Could not stage ZIP content without overwriting files.",
                )
            })?;
        let actual =
            std::io::copy(&mut entry.by_ref().take(size + 1), &mut output).map_err(|_| {
                error(
                    "invalid_zip",
                    "A ZIP entry failed decompression or integrity validation.",
                )
            })?;
        if actual != size {
            return Err(error(
                "invalid_zip",
                "A ZIP entry size does not match its contents.",
            ));
        }
        output
            .sync_all()
            .map_err(|_| error("import_io", "Could not sync extracted ZIP content."))?;
        if text {
            result.texts.push((name, target));
        } else if let Some((media_type, _)) = kind {
            result.media.push(MediaFile {
                original_name: name,
                stored_name: target.file_name().unwrap().to_string_lossy().into_owned(),
                size,
                media_type: media_type.into(),
            });
        }
    }
    if result.texts.is_empty() {
        return Err(error(
            "unsupported_format",
            "No conversation text file was found in this ZIP.",
        ));
    }
    Ok(result)
}
