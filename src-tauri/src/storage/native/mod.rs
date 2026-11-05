pub mod commands;
mod crypto;
pub(super) mod inspector;
mod models;
mod parser;
mod privacy;
#[cfg(test)]
mod tests;
use crate::storage::{StorageState, error::Result, imports::staging};
use models::*;
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::Read,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use zeroize::Zeroizing;
#[derive(Clone, Default)]
pub struct NativeState(Arc<Mutex<Runtime>>);
#[derive(Default)]
struct Runtime {
    active: Option<Arc<AtomicBool>>,
    ready: Option<Ready>,
    progress: Progress,
}
struct Ready {
    stage: staging::Stage,
    _lease: fs::File,
    vault: std::path::PathBuf,
    installation: String,
    snapshot: inspector::Snapshot,
    preview: Preview,
    created: Instant,
    plaintext_hash: String,
}
struct Active(NativeState, Arc<AtomicBool>, Instant);
impl Drop for Active {
    fn drop(&mut self) {
        if let Ok(mut r) = self.0.0.lock() {
            r.active = None;
        }
    }
}
impl Active {
    fn check(&self) -> Result<()> {
        if self.1.load(Ordering::Acquire) {
            Err(fail("native_cancelled"))
        } else if self.2.elapsed() > Duration::from_secs(600) {
            Err(fail("native_limit"))
        } else {
            Ok(())
        }
    }
}
pub enum KeyInput {
    Hex(Zeroizing<String>),
    File(Zeroizing<Vec<u8>>),
}
fn db<T>(
    s: &StorageState,
    f: impl FnOnce(&mut crate::storage::service::StorageService) -> Result<T>,
) -> Result<T> {
    s.run(false, f)
}
impl NativeState {
    fn begin(&self, allow_ready: bool) -> Result<Active> {
        let mut r = self.0.lock().map_err(|_| fail("native_busy"))?;
        if r.active.is_some() || (!allow_ready && r.ready.is_some()) {
            return Err(fail("native_busy"));
        }
        let cancel = Arc::new(AtomicBool::new(false));
        r.active = Some(cancel.clone());
        r.progress = Progress {
            phase: "preparing".into(),
            ..Default::default()
        };
        Ok(Active(self.clone(), cancel, Instant::now()))
    }
    fn update(&self, phase: &str, bytes: u64, messages: u64) {
        if let Ok(mut r) = self.0.lock() {
            r.progress.phase = phase.into();
            r.progress.bytes = bytes;
            r.progress.messages = messages;
        }
    }
    fn error<T>(&self, result: &Result<T>) {
        if let Err(e) = result {
            if let Ok(mut r) = self.0.lock() {
                r.progress.phase = if e.code == "native_cancelled" {
                    "cancelled"
                } else {
                    "failed"
                }
                .into();
                r.progress.error = Some(e.clone());
            }
        }
    }
    pub fn progress(&self) -> Result<Progress> {
        let mut r = self.0.lock().map_err(|_| fail("native_busy"))?;
        if r.ready
            .as_ref()
            .is_some_and(|p| p.created.elapsed() > Duration::from_secs(15 * 60))
        {
            r.ready.take();
            r.progress.phase = "expired".into();
        }
        Ok(r.progress.clone())
    }
    pub fn cancel(&self) -> Result<()> {
        let mut r = self.0.lock().map_err(|_| fail("native_busy"))?;
        // A preview can be published just before its worker releases Active.
        r.ready.take();
        if let Some(c) = &r.active {
            c.store(true, Ordering::Release);
            r.progress.cancel_requested = true;
        } else {
            r.progress.phase = "cancelled".into();
        }
        Ok(())
    }
    pub async fn inspect(&self, storage: StorageState, id: String) -> Result<Inspection> {
        let active = self.begin(false)?;
        let state = self.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let _active = active;
            state.update("inspecting", 0, 0);
            let result = db(&storage, |s| {
                let _lease = s.import_coordinator()?;
                let (root, installation) = s.read_vault()?;
                inspector::inspect(&s.connection, &root, &installation, &id, |n| {
                    _active.check()?;
                    state.update("inspecting", n, 0);
                    Ok(())
                })
            });
            if result.is_ok() {
                state.update("inspected", 0, 0);
            }
            let result = result.map_err(|e| _active.check().err().unwrap_or(e));
            state.error(&result);
            result
        })
        .await
        .map_err(|_| fail("native_io"))?
    }
    pub async fn verify(
        &self,
        storage: StorageState,
        snapshot_id: String,
        file_id: String,
        key: KeyInput,
        consent: bool,
    ) -> Result<Preview> {
        if !consent {
            return Err(fail("native_consent"));
        }
        let active = self.begin(false)?;
        let state = self.clone();
        tauri::async_runtime::spawn_blocking(move||{
            let _active=active;
            let result=(||->Result<Preview>{
                let (vault,installation,lease,snapshot)=db(&storage,|s|{let lease=s.import_coordinator()?;let(vault,installation)=s.read_vault()?;let snapshot=inspector::load(&s.connection,&vault,&installation,&snapshot_id)?;Ok((vault,installation,lease,snapshot))})?;
                let entry=snapshot.entries.iter().find(|f|f.id==file_id&&f.category==crate::storage::backups::models::Category::Databases).ok_or_else(||fail("native_integrity"))?;
                if entry.actual_bytes>crypto::MAX_ENCRYPTED{return Err(fail("native_limit"));}
                let source=inspector::path(&snapshot,entry)?;
                let head=inspector::verify(&source,entry,|n|{_active.check()?;state.update("verifying_source",n,0);Ok(())})?;
                let header=crypto::header(&head)?;
                if entry.original_name.rsplit('.').next()!=Some(header.format){return Err(fail("native_format"));}
                let key=match key{KeyInput::Hex(text) if header.format=="crypt15"=>crypto::hex_key(&text)?,KeyInput::File(bytes) if header.format=="crypt14"=>crypto::key_file(bytes,&header)?,_=>return Err(fail("native_key"))};
                let mut bytes=Zeroizing::new(Vec::new());bytes.try_reserve_exact(entry.actual_bytes as usize).map_err(|_|fail("native_limit"))?;
                crate::storage::paths::open_regular_file(&source)?.take(crypto::MAX_ENCRYPTED+1).read_to_end(&mut bytes).map_err(|_|fail("native_io"))?;
                if bytes.len() as u64!=entry.actual_bytes||Some(format!("{:x}",Sha256::digest(&bytes)))!=entry.sha256{return Err(fail("native_integrity"));}
                state.update("authenticating",entry.actual_bytes,0);
                let decrypted=crypto::decrypt(bytes,&header,key,||_active.check())?;
                let stage=staging::Stage::new(&vault,&installation)?;privacy::restrict(stage.root())?;privacy::restrict(&stage.payload())?;
                let path=stage.payload().join("database.sqlite");let mut output=fs::OpenOptions::new().write(true).create_new(true).open(&path).map_err(|_|fail("native_io"))?;
                crypto::plaintext(&decrypted,&mut output,|n|{_active.check()?;state.update("decompressing",n,0);Ok(())})?;output.sync_all().map_err(|_|fail("native_io"))?;drop(output);drop(decrypted);
                let plaintext_hash=hash_plain(&path,||_active.check())?;
                let source_db=parser::open(&path,_active.1.clone())?;let schema=parser::schema(&source_db)?;
                let mut matched=0;let mut missing=0;let mut verified=HashSet::new();
                let mut counts=parser::walk(&source_db,&schema,|record|{if let Some(media)=&record.media{if let Some(file)=parser::match_media(&snapshot,media){if verified.insert(file.id.clone()){inspector::verify(&inspector::path(&snapshot,file)?,file,|_|_active.check())?;}matched+=1;}else{missing+=1;}}Ok(())},|n|{_active.check()?;state.update("parsing",0,n);Ok(())})?;
                counts.media_matched=matched;counts.media_missing=missing;
                let mut warnings=snapshot.manifest.warnings.clone();warnings.push("Contact names are not inferred; identifiers and LIDs remain separate. Unsupported/system action details are not manufactured.".into());
                let preview=Preview{token:uuid::Uuid::new_v4().to_string(),snapshot_id,file_id,format:header.format.into(),schema:schema.name().into(),parser_version:PARSER_VERSION,counts,warnings,archive_id:None};
                drop(source_db);let ready=Ready{stage,_lease:lease,vault,installation,snapshot,preview:preview.clone(),created:Instant::now(),plaintext_hash};
                {let mut runtime=state.0.lock().map_err(|_|fail("native_busy"))?;_active.check()?;runtime.ready=Some(ready);}state.update("key_verified",0,preview.counts.messages);Ok(preview)
            })();let result=result.map_err(|e|_active.check().err().unwrap_or(e));state.error(&result);result
        }).await.map_err(|_|fail("native_io"))?
    }
    pub async fn import(
        &self,
        storage: StorageState,
        token: String,
        consent: bool,
    ) -> Result<Preview> {
        if !consent {
            return Err(fail("native_consent"));
        }
        let active = self.begin(true)?;
        let state = self.clone();
        tauri::async_runtime::spawn_blocking(move||{let _active=active;
            let result=(||->Result<Preview>{
                let mut ready=state.0.lock().map_err(|_|fail("native_busy"))?.ready.take().filter(|r|r.preview.token==token&&r.created.elapsed()<Duration::from_secs(15*60)).ok_or_else(||fail("native_expired"))?;
                let selected=ready.snapshot.entries.iter().find(|f|f.id==ready.preview.file_id).ok_or_else(||fail("native_integrity"))?; inspector::verify(&inspector::path(&ready.snapshot,selected)?,selected,|_|_active.check())?;
                let path=ready.stage.payload().join("database.sqlite");if hash_plain(&path,||_active.check())?!=ready.plaintext_hash{return Err(fail("native_integrity"));}
                let source=parser::open(&path,_active.1.clone())?;let schema=parser::schema(&source)?;
                let id=uuid::Uuid::new_v4().to_string();let destination=ready.vault.join("archives").join(&id);
                db(&storage,|s|{let existing:i64=sql(s.connection.query_row("SELECT count(*) FROM archives WHERE native_snapshot_id=?1 AND native_parser_version=?2",rusqlite::params![ready.preview.snapshot_id,PARSER_VERSION],|r|r.get(0)))?;if existing>0{return Err(fail("native_duplicate"));}
                    sql(s.connection.execute("INSERT INTO import_jobs(id,vault_path,storage_path,installation_id) VALUES(?1,?2,?3,?4)",rusqlite::params![id,ready.vault.to_string_lossy(),format!("archives/{id}"),ready.installation]))?;Ok(())})?;
                crate::storage::paths::verify_ancestors(destination.parent().ok_or_else(||fail("native_io"))?,false)?;
                fs::create_dir(&destination).map_err(|_|fail("native_io"))?;
                staging::write_owner(&destination,staging::FINAL_MARKER,&id,&ready.installation)?;
                let saved=db(&storage,|s|{
                    let tx=sql(s.connection.transaction())?;
                    let title=format!("{} native backup",ready.snapshot.manifest.source.base());
                    let filename=ready.snapshot.entries.iter().find(|f|f.id==ready.preview.file_id).ok_or_else(||fail("native_integrity"))?.original_name.clone();
                    sql(tx.execute("INSERT INTO archives(id,title,source_type,storage_path,original_filename,import_status,native_snapshot_id,native_file_id,native_format,native_source_app,native_parser_version,native_schema,native_diagnostics_json) VALUES(?1,?2,'chat-export',?3,?4,'complete',?5,?6,?7,?8,?9,?10,?11)",rusqlite::params![id,title,format!("archives/{id}"),filename,ready.preview.snapshot_id,ready.preview.file_id,ready.preview.format,ready.snapshot.manifest.source.key(),PARSER_VERSION,ready.preview.schema,serde_json::to_string(&serde_json::json!({"warnings":ready.preview.warnings,"counts":ready.preview.counts,"schema":ready.preview.schema,"parserVersion":PARSER_VERSION})).map_err(|_|fail("native_database"))?]))?;
                    let mut conversations:HashMap<String,(String,HashSet<String>)>=HashMap::new();let mut media_ids:HashMap<String,String>=HashMap::new();let mut sequence=0i64;
                    let mut counts=parser::walk(&source,&schema,|record|{
                        _active.check()?;
                        if !conversations.contains_key(&record.jid){let cid=uuid::Uuid::new_v4().to_string();sql(tx.execute("INSERT INTO conversations(id,archive_id,title) VALUES(?1,?2,?3)",rusqlite::params![cid,id,record.title]))?;conversations.insert(record.jid.clone(),(cid,HashSet::new()));}
                        let (conversation,participants)=conversations.get_mut(&record.jid).unwrap();if let Some(sender)=&record.sender{participants.insert(sender.clone());if participants.len()>10000{return Err(fail("native_limit"));}}
                        let mid=uuid::Uuid::new_v4().to_string();let timestamp=record.timestamp.map(|v|format!("{v} (Unix milliseconds, UTC)"));
                        sql(tx.execute("INSERT INTO messages(id,conversation_id,sender,message_text,timestamp,original_timestamp,classification,sequence_number,native_id,native_direction,native_timestamp_ms,native_type,native_reply_key) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",rusqlite::params![mid,conversation.as_str(),record.sender,record.text,record.timestamp.filter(|t|*t>=0),timestamp,record.classification,sequence,record.key,record.direction,record.timestamp,record.kind,record.reply]))?;sequence+=1;
                        if let Some(media)=record.media{
                            let matched=parser::match_media(&ready.snapshot,&media);
                            let (media_id,relative,filename,size)=if let Some(file)=matched{
                                let media_id=if let Some(id)=media_ids.get(&file.id){id.clone()}else{
                                    inspector::verify(&inspector::path(&ready.snapshot,file)?,file,|_|_active.check())?;
                                    let media_id=uuid::Uuid::new_v4().to_string();let ext=std::path::Path::new(&file.original_name).extension().and_then(|x|x.to_str()).filter(|s|s.len()<=10&&s.bytes().all(|b|b.is_ascii_alphanumeric())).unwrap_or("bin");
                                    let relative=format!("archives/{id}/media/{media_id}.{ext}");
                                    sql(tx.execute("INSERT INTO archive_media(id,archive_id,original_filename,storage_path,media_type,file_size) VALUES(?1,?2,?3,?4,?5,?6)",rusqlite::params![media_id,id,file.original_name,relative,media.mime.as_deref().unwrap_or("application/octet-stream"),file.actual_bytes as i64]))?;
                                    sql(tx.execute("INSERT INTO native_media_links(media_id,snapshot_id,backup_file_id) VALUES(?1,?2,?3)",rusqlite::params![media_id,ready.preview.snapshot_id,file.id]))?;media_ids.insert(file.id.clone(),media_id.clone());media_id
                                };
                                let relative:String=sql(tx.query_row("SELECT storage_path FROM archive_media WHERE id=?1",[&media_id],|r|r.get(0)))?;
                                (Some(media_id),Some(relative),file.original_name.clone(),Some(file.actual_bytes as i64))
                            }else{(None,None,media.path.as_deref().and_then(|p|p.rsplit('/').next()).unwrap_or("Unresolved native attachment").to_owned(),media.size.map(|s|s as i64))};
                            sql(tx.execute("INSERT INTO attachments(id,message_id,media_type,storage_path,file_size,original_filename,archive_media_id,is_missing) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",rusqlite::params![uuid::Uuid::new_v4().to_string(),mid,media.mime,relative,size,filename,media_id,matched.is_none()]))?;
                        }Ok(())
                    },|n|{_active.check()?;state.update("importing",0,n);Ok(())})?;
                    sql(tx.execute("UPDATE messages AS m SET native_reply_id=(SELECT min(q.id) FROM messages q WHERE q.conversation_id=m.conversation_id AND q.native_id=m.native_reply_key HAVING count(*)=1) WHERE native_reply_key IS NOT NULL AND conversation_id IN (SELECT id FROM conversations WHERE archive_id=?1)",[&id]))?;
                    for(cid,participants)in conversations.values(){sql(tx.execute("UPDATE conversations SET participants_json=?2 WHERE id=?1",rusqlite::params![cid,serde_json::to_string(participants).map_err(|_|fail("native_database"))?]))?;crate::storage::viewer::search::index_conversation(&tx,cid)?;}
                    counts.media_matched=ready.preview.counts.media_matched;counts.media_missing=ready.preview.counts.media_missing;
                    sql(tx.execute("UPDATE archives SET message_count=?2,media_count=?3,missing_media_count=?4 WHERE id=?1",rusqlite::params![id,counts.messages as i64,media_ids.len() as i64,counts.media_missing as i64]))?;
                    _active.check()?;sql(tx.execute("DELETE FROM import_jobs WHERE id=?1",[&id]))?;sql(tx.commit())?;Ok(counts)
                });
                if saved.is_err(){let _=staging::cleanup_owned(&destination,staging::FINAL_MARKER,&id,&ready.installation);}
                ready.preview.counts=saved?;ready.preview.archive_id=Some(id);state.update("completed",0,ready.preview.counts.messages);Ok(ready.preview.clone())
            })();let result=result.map_err(|e|_active.check().err().unwrap_or(e));state.error(&result);result
        }).await.map_err(|_|fail("native_io"))?
    }
}
fn hash_plain(path: &std::path::Path, mut check: impl FnMut() -> Result<()>) -> Result<String> {
    let mut f = crate::storage::paths::open_regular_file(path)?;
    let mut hash = Sha256::new();
    let mut buffer = Zeroizing::new(vec![0u8; 128 * 1024]);
    let mut bytes = 0;
    loop {
        check()?;
        let n = f.read(&mut buffer).map_err(|_| fail("native_io"))?;
        if n == 0 {
            break;
        }
        bytes += n as u64;
        if bytes > crypto::MAX_PLAIN {
            return Err(fail("native_limit"));
        }
        hash.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
