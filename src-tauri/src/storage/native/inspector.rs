use super::{crypto, models::*};
use crate::storage::{
    backups::{
        files,
        models::{FileEntry, Manifest},
        repository,
    },
    error::Result,
    paths,
};
use rusqlite::{Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    io::Read,
    path::{Path, PathBuf},
};
pub struct Snapshot {
    pub root: PathBuf,
    pub manifest: Manifest,
    pub entries: Vec<FileEntry>,
}
pub fn load(c: &Connection, vault: &Path, installation: &str, id: &str) -> Result<Snapshot> {
    uuid::Uuid::parse_str(id).map_err(|_| fail("native_integrity"))?;
    let complete: i64 = sql(c.query_row(
        "SELECT count(*) FROM backup_snapshots WHERE id=?1 AND status='complete'",
        [id],
        |r| r.get(0),
    ))?;
    if complete != 1 {
        return Err(fail("native_integrity"));
    }
    let root = vault.join("backups").join(id);
    files::verify_owner(&root, id, installation).map_err(|_| fail("native_integrity"))?;
    let manifest = files::read_manifest(&root).map_err(|_| fail("native_integrity"))?;
    if manifest.schema_version != 1
        || manifest.snapshot_id != id
        || !matches!(
            manifest.phase.as_str(),
            "completed" | "completed_with_warnings"
        )
    {
        return Err(fail("native_integrity"));
    }
    let entries = repository::entries(c, id, 0, 50_001).map_err(|_| fail("native_integrity"))?;
    if entries.len() > 50_000
        || entries.len() != manifest.files.len()
        || !entries.iter().zip(&manifest.files).all(|(a, b)| {
            a.id == b.id
                && a.sha256 == b.sha256
                && a.local_relative == b.local_relative
                && a.original_name == b.original_name
                && a.actual_bytes == b.actual_bytes
                && a.category == b.category
        })
    {
        return Err(fail("native_integrity"));
    }
    Ok(Snapshot {
        root,
        manifest,
        entries,
    })
}
pub fn path(snapshot: &Snapshot, entry: &FileEntry) -> Result<PathBuf> {
    files::join(&snapshot.root, &entry.local_relative).map_err(|_| fail("native_integrity"))
}
pub fn verify(
    path: &Path,
    entry: &FileEntry,
    mut check: impl FnMut(u64) -> Result<()>,
) -> Result<[u8; 4096]> {
    let mut f = paths::open_regular_file(path).map_err(|_| fail("native_integrity"))?;
    if f.metadata().map_err(|_| fail("native_integrity"))?.len() != entry.actual_bytes
        || entry.state != "verified"
        || entry.local_verification != "verified"
    {
        return Err(fail("native_integrity"));
    }
    let mut buffer = vec![0u8; 256 * 1024];
    let mut hash = Sha256::new();
    let mut size = 0;
    let mut head = [0; 4096];
    loop {
        check(size)?;
        let n = f.read(&mut buffer).map_err(|_| fail("native_integrity"))?;
        if n == 0 {
            break;
        }
        if size == 0 {
            let k = n.min(head.len());
            head[..k].copy_from_slice(&buffer[..k]);
        }
        size += n as u64;
        if size > entry.actual_bytes {
            return Err(fail("native_integrity"));
        }
        hash.update(&buffer[..n]);
    }
    if size != entry.actual_bytes || Some(format!("{:x}", hash.finalize())) != entry.sha256 {
        return Err(fail("native_integrity"));
    }
    Ok(head)
}
pub fn inspect(
    c: &Connection,
    vault: &Path,
    installation: &str,
    id: &str,
    mut check: impl FnMut(u64) -> Result<()>,
) -> Result<Inspection> {
    let snapshot = load(c, vault, installation, id)?;
    let (source, created): (Option<String>, i64) = sql(c.query_row(
        "SELECT source_app,created_at FROM backup_snapshots WHERE id=?1",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ))?;
    let has_native:bool=sql(c.query_row("SELECT EXISTS(SELECT 1 FROM pragma_table_info('archives') WHERE name='native_snapshot_id')",[],|r|r.get(0)))?;
    let existing_archive = if has_native {
        sql(c
            .query_row(
                "SELECT id FROM archives WHERE native_snapshot_id=?1 AND native_parser_version=?2",
                rusqlite::params![id, PARSER_VERSION],
                |r| r.get(0),
            )
            .optional())?
    } else {
        None
    };
    let local_integrity: String = sql(c.query_row(
        "SELECT local_integrity FROM backup_snapshots WHERE id=?1",
        [id],
        |r| r.get(0),
    ))?;
    let mut candidates = vec![];
    let mut media: std::collections::BTreeMap<String, (usize, u64)> = Default::default();
    let mut missing = 0;
    let mut hashes = HashMap::new();
    for entry in &snapshot.entries {
        check(0)?;
        let p = path(&snapshot, entry)?;
        if paths::open_regular_file(&p)
            .and_then(|f| {
                let m = f.metadata().map_err(|_| fail("native_integrity"))?;
                if m.len() != entry.actual_bytes {
                    return Err(fail("native_integrity"));
                }
                Ok(m)
            })
            .is_err()
        {
            missing += 1;
        }
        if entry.category != crate::storage::backups::models::Category::Databases {
            let value = media.entry(entry.category.key().into()).or_default();
            value.0 += 1;
            value.1 += entry.actual_bytes;
            continue;
        }
        let result = verify(&p, entry, &mut check);
        if result
            .as_ref()
            .err()
            .is_some_and(|e| matches!(e.code, "native_cancelled" | "native_limit"))
        {
            return Err(result.err().unwrap());
        }
        let hash_verified = result.is_ok();
        let h = result
            .ok()
            .and_then(|h| crypto::header(&h).ok())
            .filter(|h| entry.actual_bytes > h.offset as u64 + 32);
        let ext = entry
            .original_name
            .rsplit('.')
            .next()
            .unwrap_or("")
            .to_lowercase();
        let format = h
            .as_ref()
            .map(|h| h.format.to_owned())
            .unwrap_or_else(|| format!("unrecognized ({ext})"));
        let standalone = if h.is_some() {
            crypto::standalone_checksum(&p, &mut check)?
        } else {
            false
        };
        let capability = if !hash_verified {
            "INCOMPLETE_BACKUP"
        } else if h.is_some() && !standalone {
            "UNSUPPORTED_FORMAT"
        } else if h.is_some() && format == ext {
            "SUPPORTED_AND_KEY_REQUIRED"
        } else if matches!(ext.as_str(), "crypt14" | "crypt15") {
            "NEEDS_INVESTIGATION"
        } else {
            "UNSUPPORTED_FORMAT"
        };
        let duplicate = entry
            .sha256
            .as_ref()
            .and_then(|hash| hashes.insert(hash.clone(), entry.id.clone()));
        candidates.push(Candidate{file_id:entry.id.clone(),filename:entry.original_name.clone(),bytes:entry.actual_bytes,format,key_requirement:if h.as_ref().is_some_and(|h|h.format=="crypt14"){"Matching local crypt14 key file (raw 131-byte or exact serialized 158-byte byte array)"}else{"Matching 64-character hexadecimal crypt15 root key; passwords/passkeys and other key modes unsupported"}.into(),capability:capability.into(),hash_verified,duplicate_of:duplicate,issue:if !hash_verified {Some("Missing, incomplete, changed or unverified candidate".into())}else if h.is_some()&&!standalone{Some("Standalone checksum envelope required; multifile/incremental payloads are unsupported.".into())}else{None}});
    }
    check(0)?;
    let capability = if missing > 0
        || !snapshot.manifest.inventory_complete
        || local_integrity == "failed"
        || candidates.iter().any(|c| !c.hash_verified)
    {
        "INCOMPLETE_BACKUP"
    } else if candidates.is_empty() {
        "NO_MESSAGE_DATABASE"
    } else if candidates
        .iter()
        .any(|c| c.capability == "SUPPORTED_AND_KEY_REQUIRED")
    {
        "SUPPORTED_AND_KEY_REQUIRED"
    } else {
        if candidates
            .iter()
            .all(|c| c.capability == "UNSUPPORTED_FORMAT")
        {
            "UNSUPPORTED_FORMAT"
        } else {
            "NEEDS_INVESTIGATION"
        }
    };
    Ok(Inspection {
        local_integrity,
        snapshot_id: id.into(),
        source_app: source,
        created_at: created as u64,
        phase: snapshot.manifest.phase,
        inventory_complete: snapshot.manifest.inventory_complete,
        warnings: snapshot.manifest.warnings,
        capability: capability.into(),
        candidates,
        media: media.into_iter().map(|(k, (n, b))| (k, n, b)).collect(),
        missing_files: missing,
        total_files: snapshot.entries.len(),
        existing_archive,
    })
}
