use super::{Result, failure, id, models::*};
use crate::storage::{error::StorageError, imports::staging, paths, service::StorageService};
use rusqlite::OptionalExtension;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
    time::{Duration, Instant},
};
pub struct Grant {
    pub media_id: String,
    pub created: Instant,
}
pub(crate) struct Resolved {
    pub file: File,
    pub path: PathBuf,
    pub metadata: MediaMetadata,
}
fn unavailable() -> StorageError {
    failure(
        "media_missing",
        "This attachment is missing or unavailable. Reconnect its vault drive and retry.",
    )
}
fn unsupported() -> StorageError {
    failure(
        "media_unsupported",
        "This file is not a supported, safely identified image, recording, or document.",
    )
}
fn dimensions(bytes: &[u8], mime: &str) -> (Option<u32>, Option<u32>) {
    match mime {
        "image/png" if bytes.len() >= 24 => (
            Some(u32::from_be_bytes(bytes[16..20].try_into().unwrap())),
            Some(u32::from_be_bytes(bytes[20..24].try_into().unwrap())),
        ),
        "image/gif" if bytes.len() >= 10 => (
            Some(u16::from_le_bytes(bytes[6..8].try_into().unwrap()) as u32),
            Some(u16::from_le_bytes(bytes[8..10].try_into().unwrap()) as u32),
        ),
        "image/webp" if bytes.len() >= 30 && &bytes[12..16] == b"VP8X" => (
            Some(1 + u32::from_le_bytes([bytes[24], bytes[25], bytes[26], 0])),
            Some(1 + u32::from_le_bytes([bytes[27], bytes[28], bytes[29], 0])),
        ),
        "image/jpeg" => {
            let mut i = 2;
            while i + 9 < bytes.len() {
                if bytes[i] != 0xff {
                    break;
                }
                let marker = bytes[i + 1];
                if marker == 0xd9 || marker == 0xda {
                    break;
                }
                if marker == 0xff {
                    i += 1;
                    continue;
                }
                let length = u16::from_be_bytes([bytes[i + 2], bytes[i + 3]]) as usize;
                if length < 2 {
                    break;
                }
                if matches!(marker,0xc0..=0xc3|0xc5..=0xc7|0xc9..=0xcb|0xcd..=0xcf) {
                    return (
                        Some(u16::from_be_bytes([bytes[i + 7], bytes[i + 8]]) as u32),
                        Some(u16::from_be_bytes([bytes[i + 5], bytes[i + 6]]) as u32),
                    );
                }
                i += length + 2;
            }
            (None, None)
        }
        _ => (None, None),
    }
}
fn identify(extension: &str, b: &[u8]) -> Result<(&'static str, &'static str)> {
    let riff = b.len() >= 12 && &b[..4] == b"RIFF";
    let iso = b.len() >= 12 && &b[4..8] == b"ftyp";
    let value = match extension {
        "png" if b.starts_with(b"\x89PNG\r\n\x1a\n") => ("image/png", "image"),
        "jpg" | "jpeg" if b.starts_with(&[0xff, 0xd8, 0xff]) => ("image/jpeg", "image"),
        "gif" if b.starts_with(b"GIF87a") || b.starts_with(b"GIF89a") => ("image/gif", "image"),
        "webp" if riff && &b[8..12] == b"WEBP" => ("image/webp", "image"),
        "mp4" | "m4v" if iso => ("video/mp4", "video"),
        "webm" if b.starts_with(&[0x1a, 0x45, 0xdf, 0xa3]) => ("video/webm", "video"),
        "m4a" if iso => ("audio/mp4", "audio"),
        "ogg" | "opus" if b.starts_with(b"OggS") => ("audio/ogg", "audio"),
        "wav" if riff && &b[8..12] == b"WAVE" => ("audio/wav", "audio"),
        "mp3" if b.starts_with(b"ID3") || (b.len() > 1 && b[0] == 0xff && b[1] & 0xe0 == 0xe0) => {
            ("audio/mpeg", "audio")
        }
        "aac" if b.len() > 1 && b[0] == 0xff && b[1] & 0xf6 == 0xf0 => ("audio/aac", "audio"),
        "pdf" if b.starts_with(b"%PDF-") => ("application/pdf", "document"),
        "docx" | "xlsx" | "pptx" if b.starts_with(b"PK\x03\x04") => {
            ("application/octet-stream", "document")
        }
        "doc" | "xls" | "ppt"
            if b.starts_with(&[0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1]) =>
        {
            ("application/octet-stream", "document")
        }
        "txt" if !b.contains(&0) && !b.starts_with(b"MZ") => ("text/plain", "document"),
        _ => return Err(unsupported()),
    };
    Ok(value)
}
impl StorageService {
    pub(crate) fn resolve_media(&self, media_id: &str) -> Result<Resolved> {
        id(media_id)?;
        let (archive,relative,filename,size):(String,String,String,i64)=self.connection.query_row("SELECT a.id,am.storage_path,am.original_filename,am.file_size FROM archive_media am JOIN archives a ON a.id=am.archive_id WHERE am.id=?1 AND a.import_status='complete' AND EXISTS(SELECT 1 FROM conversations c WHERE c.archive_id=a.id)",[media_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional().map_err(|_|StorageError::database())?.ok_or_else(unavailable)?;
        uuid::Uuid::parse_str(&archive).map_err(|_| unsupported())?;
        let components: Vec<_> = relative.split('/').collect();
        if components.len() != 4
            || components[0] != "archives"
            || components[1] != archive
            || components[2] != "media"
            || components[3].contains(['\\', ':'])
        {
            return Err(failure(
                "media_denied",
                "The attachment storage reference is outside its authorized archive.",
            ));
        }
        let name = std::path::Path::new(components[3]);
        let stem = name
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(unsupported)?;
        uuid::Uuid::parse_str(stem).map_err(|_| unsupported())?;
        let extension = name
            .extension()
            .and_then(|s| s.to_str())
            .ok_or_else(unsupported)?
            .to_ascii_lowercase();
        if size < 0 || size > 64 * 1024 * 1024 {
            return Err(failure(
                "media_limit",
                "The attachment exceeds the supported playback size.",
            ));
        }
        let (vault, installation) = self.read_vault()?;
        let root = vault.join("archives").join(&archive);
        staging::verify_archive_owner(&root, &archive, &installation).map_err(|_| {
            failure(
                "media_denied",
                "Archive ownership could not be verified. The attachment was not opened.",
            )
        })?;
        let native: Option<(String,String)> = self.connection.query_row("SELECT n.snapshot_id,n.backup_file_id FROM native_media_links n JOIN archives a ON a.id=?2 WHERE n.media_id=?1 AND a.native_snapshot_id=n.snapshot_id",rusqlite::params![media_id,archive],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(|_|StorageError::database())?;
        let (path, mut file) = if let Some((snapshot_id, file_id)) = native {
            let snapshot = crate::storage::native::inspector::load(
                &self.connection,
                &vault,
                &installation,
                &snapshot_id,
            )
            .map_err(|_| unavailable())?;
            let entry = snapshot
                .entries
                .iter()
                .find(|f| {
                    f.id == file_id && f.actual_bytes == size as u64 && f.original_name == filename
                })
                .ok_or_else(unavailable)?;
            let path = crate::storage::native::inspector::path(&snapshot, entry)
                .map_err(|_| unavailable())?;
            crate::storage::native::inspector::verify(&path, entry, |_| Ok(()))
                .map_err(|_| unavailable())?;
            let file = paths::open_regular_file(&path).map_err(|_| unavailable())?;
            (path, file)
        } else {
            let expected = vault.join(&relative);
            let parent = paths::canonical(expected.parent().ok_or_else(unavailable)?)
                .map_err(|_| unavailable())?;
            let path = parent.join(expected.file_name().ok_or_else(unavailable)?);
            if !path.starts_with(&root) || path != expected {
                return Err(failure(
                    "media_denied",
                    "The attachment path is not within its authorized archive.",
                ));
            }
            let file = paths::open_regular_file(&path).map_err(|_| unavailable())?;
            (path, file)
        };
        if file.metadata().map_err(|_| unavailable())?.len() != size as u64 {
            return Err(failure(
                "media_changed",
                "This attachment changed after import. It was not opened.",
            ));
        }
        let mut head = vec![0; (size as usize).min(65536)];
        file.read_exact(&mut head).map_err(|_| unavailable())?;
        let (mime, kind) = identify(&extension, &head)?;
        let (width, height) = dimensions(&head, mime);
        if kind == "image"
            && (size > 16 * 1024 * 1024
                || width
                    .zip(height)
                    .is_some_and(|(w, h)| w == 0 || h == 0 || (w as u64) * (h as u64) > 25_000_000))
        {
            return Err(failure(
                "media_limit",
                "This image exceeds the viewer's file size or pixel limit.",
            ));
        }
        if mime == "image/jpeg" && width.is_none() {
            return Err(unsupported());
        }
        file.seek(SeekFrom::Start(0)).map_err(|_| unavailable())?;
        Ok(Resolved {
            file,
            path,
            metadata: MediaMetadata {
                id: media_id.to_owned(),
                filename,
                size_bytes: size as u64,
                mime: mime.to_owned(),
                kind,
                width,
                height,
            },
        })
    }
    pub fn media_metadata(&self, media_id: &str) -> Result<MediaMetadata> {
        Ok(self.resolve_media(media_id)?.metadata)
    }
    pub fn authorize_media(&mut self, media_id: &str) -> Result<MediaAccess> {
        let metadata = self.media_metadata(media_id)?;
        if metadata.kind == "document" {
            return Err(failure(
                "media_document",
                "Use the explicit Open document action to open this file with its registered application.",
            ));
        }
        self.media_grants
            .retain(|_, g| g.created.elapsed() < Duration::from_secs(30 * 60));
        if self.media_grants.len() >= 128 {
            self.media_grants.clear();
        }
        let token = uuid::Uuid::new_v4().to_string();
        self.media_grants.insert(
            token.clone(),
            Grant {
                media_id: media_id.to_owned(),
                created: Instant::now(),
            },
        );
        Ok(MediaAccess { token, metadata })
    }
    pub fn revoke_media(&mut self, token: &str) -> Result<()> {
        self.media_grants.remove(token);
        Ok(())
    }
    pub fn open_document(&self, media_id: &str) -> Result<()> {
        let resolved = self.resolve_media(media_id)?;
        if resolved.metadata.kind != "document" {
            return Err(failure(
                "media_denied",
                "Only supported documents can be opened with a registered application.",
            ));
        }
        open_registered(&resolved.path)
    }
    pub(crate) fn granted_media(&self, token: &str) -> Result<Resolved> {
        let grant = self
            .media_grants
            .get(token)
            .filter(|g| g.created.elapsed() < Duration::from_secs(30 * 60))
            .ok_or_else(|| {
                failure(
                    "media_expired",
                    "The private media authorization expired. Load this attachment again.",
                )
            })?;
        self.resolve_media(&grant.media_id)
    }
}
#[cfg(windows)]
fn open_registered(path: &std::path::Path) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};
    let path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let verb: Vec<u16> = "open".encode_utf16().chain(Some(0)).collect();
    // SAFETY: nul-terminated validated local path, literal verb, no arguments/shell
    // command composition. Called only by the explicit document IPC action.
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            path.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    if result as isize <= 32 {
        return Err(failure(
            "document_open_failed",
            "No registered application could open this document.",
        ));
    }
    Ok(())
}
#[cfg(not(windows))]
fn open_registered(_: &std::path::Path) -> Result<()> {
    Err(failure(
        "document_open_failed",
        "Registered document opening is currently implemented for Windows only.",
    ))
}
