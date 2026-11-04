use super::models::*;
use crate::storage::service::StorageService;
use rusqlite::{Connection, params};
pub fn start(
    c: &mut Connection,
    id: &str,
    source: Source,
    name: Option<&str>,
    files: &[super::planner::PlannedFile],
    warnings: &[String],
) -> Result<()> {
    let tx = c.transaction()?;
    let now = crate::devices::models::now() as i64;
    tx.execute("INSERT INTO backup_snapshots(id,status,storage_path,created_at,source_app,device_name,phase,file_count,warnings_json) VALUES(?1,'in-progress',?2,?3,?4,?5,'preparing',?6,?7)",params![id,format!("backups/{id}"),now,source.key(),name,files.len() as i64,serde_json::to_string(warnings).map_err(|_|fail("database"))?])?;
    tx.execute("INSERT INTO backup_jobs(id,snapshot_id,phase,updated_at,stage_relative) VALUES(?1,?1,'preparing',?2,?3)",params![id,now,format!("temp/backup-{id}")])?;
    for file in files {
        let f = &file.entry;
        tx.execute("INSERT INTO backup_files(id,snapshot_id,category,source_relative,local_relative,original_name,expected_size) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![f.id,id,f.category.key(),f.source_relative,f.local_relative,f.original_name,f.expected_size.map(|v|v as i64)])?;
    }
    tx.commit()?;
    Ok(())
}
pub fn progress(c: &Connection, p: &Progress) -> Result<()> {
    let Some(id) = &p.snapshot_id else {
        return Ok(());
    };
    c.execute("UPDATE backup_jobs SET phase=?2,copied_bytes=?3,verified_bytes=?4,error_code=?5,updated_at=?6 WHERE snapshot_id=?1",params![id,p.phase,p.copied_bytes as i64,p.verified_bytes as i64,p.error.as_ref().map(|e|e.code.as_str()),p.updated_at as i64])?;
    c.execute("UPDATE backup_snapshots SET phase=?2,total_bytes=?3,verified_bytes=?4,successful_count=?5,failed_count=?6 WHERE id=?1",params![id,p.phase,p.copied_bytes as i64,p.verified_bytes as i64,p.successful_files as i64,p.failed_files as i64])?;
    Ok(())
}
pub fn file(c: &Connection, snapshot: &str, f: &FileEntry) -> Result<()> {
    c.execute("UPDATE backup_files SET actual_bytes=?3,sha256=?4,local_verification=?5,source_verification=?6,size_validation=?7,state=?8,error_code=?9 WHERE snapshot_id=?1 AND id=?2",params![snapshot,f.id,f.actual_bytes as i64,f.sha256,f.local_verification,f.source_verification,f.size_validation,f.state,f.error_code])?;
    Ok(())
}
pub fn terminal(c: &mut Connection, p: &Progress, warnings: &[String]) -> Result<()> {
    let id = p.snapshot_id.as_ref().ok_or_else(|| fail("database"))?;
    let tx = c.transaction()?;
    progress(&tx, p)?;
    let complete = matches!(p.phase.as_str(), "completed" | "completed_with_warnings");
    let status = if complete {
        "complete"
    } else if p.phase == "cancelled" {
        "cancelled"
    } else {
        "failed"
    };
    tx.execute("UPDATE backup_snapshots SET total_bytes=(SELECT coalesce(sum(actual_bytes),0) FROM backup_files WHERE snapshot_id=?1),status=?2,completed_at=?3,manifest_path=?4,warnings_json=?5,local_integrity=?6,local_verified_at=?3 WHERE id=?1",params![id,status,complete.then_some(p.updated_at as i64),complete.then(||format!("backups/{id}/manifest.json")),serde_json::to_string(warnings).map_err(|_|fail("database"))?,if complete{"verified"}else{"incomplete"}])?;
    tx.commit()?;
    Ok(())
}
pub fn list(
    c: &Connection,
    vault: &std::path::Path,
    offset: usize,
    limit: usize,
) -> Result<Vec<Snapshot>> {
    let mut stmt=c.prepare("SELECT id,source_app,device_name,phase,created_at,completed_at,file_count,successful_count,failed_count,skipped_count,total_bytes,verified_bytes,manifest_path,local_integrity,local_verified_at,warnings_json,storage_path FROM backup_snapshots ORDER BY created_at DESC,id LIMIT ?1 OFFSET ?2")?;
    let rows = stmt.query_map(
        params![limit.clamp(1, 50) as i64, offset.min(1_000_000) as i64],
        |r| {
            let warnings: String = r.get(15)?;
            let path: String = r.get(16)?;
            Ok(Snapshot {
                id: r.get(0)?,
                source_app: r.get(1)?,
                device_name: r.get(2)?,
                phase: r.get(3)?,
                created_at: r.get::<_, i64>(4)? as u64,
                completed_at: r.get::<_, Option<i64>>(5)?.map(|v| v as u64),
                file_count: r.get::<_, i64>(6)? as usize,
                successful_count: r.get::<_, i64>(7)? as usize,
                failed_count: r.get::<_, i64>(8)? as usize,
                skipped_count: r.get::<_, i64>(9)? as usize,
                total_bytes: r.get::<_, i64>(10)? as u64,
                verified_bytes: r.get::<_, i64>(11)? as u64,
                manifest_path: r.get(12)?,
                local_integrity: r.get(13)?,
                local_verified_at: r.get::<_, Option<i64>>(14)?.map(|v| v as u64),
                warnings: serde_json::from_str(&warnings).unwrap_or_default(),
                destination: vault.join(path).to_string_lossy().into_owned(),
            })
        },
    )?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}
pub fn entries(c: &Connection, id: &str, offset: usize, limit: usize) -> Result<Vec<FileEntry>> {
    let mut stmt=c.prepare("SELECT id,category,source_relative,local_relative,original_name,expected_size,actual_bytes,sha256,local_verification,source_verification,size_validation,state,error_code FROM backup_files WHERE snapshot_id=?1 ORDER BY rowid LIMIT ?2 OFFSET ?3")?;
    let rows = stmt.query_map(
        params![id, limit.min(50_001) as i64, offset.min(1_000_000) as i64],
        |r| {
            let category: String = r.get(1)?;
            Ok(FileEntry {
                id: r.get(0)?,
                category: match category.as_str() {
                    "databases" => Category::Databases,
                    "images" => Category::Images,
                    "videos" => Category::Videos,
                    "documents" => Category::Documents,
                    "audio" => Category::Audio,
                    "voice_notes" => Category::VoiceNotes,
                    _ => Category::Other,
                },
                source_relative: r.get(2)?,
                local_relative: r.get(3)?,
                original_name: r.get(4)?,
                expected_size: r.get::<_, Option<i64>>(5)?.map(|v| v as u64),
                actual_bytes: r.get::<_, i64>(6)? as u64,
                sha256: r.get(7)?,
                local_verification: r.get(8)?,
                source_verification: r.get(9)?,
                size_validation: r.get(10)?,
                state: r.get(11)?,
                error_code: r.get(12)?,
            })
        },
    )?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}
impl StorageService {
    pub(crate) fn backup_context(&self) -> Result<super::Context> {
        let lease = self.import_coordinator().map_err(|_| fail("busy"))?;
        let (root, installation) = self.read_vault()?;
        for name in ["backups", "temp"] {
            crate::storage::paths::write_probe(&root.join(name))?;
        }
        Ok(super::Context {
            root,
            installation,
            _lease: lease,
        })
    }
    pub(crate) fn backup_recovery(&mut self) -> Result<()> {
        let lease = match self.import_coordinator() {
            Ok(v) => v,
            Err(_) => return Ok(()),
        };
        let ids: Vec<String> = {
            let mut stmt=self.connection.prepare("SELECT snapshot_id FROM backup_jobs WHERE phase IN ('preparing','copying','verifying','committing')")?;
            stmt.query_map([], |r| r.get(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };
        for id in ids {
            super::validate_id(&id)?;
            self.connection.execute("UPDATE backup_snapshots SET status='failed',phase='interrupted',local_integrity='incomplete' WHERE id=?1",[&id])?;
            self.connection.execute("UPDATE backup_jobs SET phase='interrupted',error_code='interrupted',updated_at=?2 WHERE snapshot_id=?1",params![id,crate::devices::models::now() as i64])?;
        }
        self.connection.execute("UPDATE backup_snapshots SET local_integrity='verification_interrupted' WHERE local_integrity='verifying'", [])?;
        let _ = lease;
        Ok(())
    }
}
