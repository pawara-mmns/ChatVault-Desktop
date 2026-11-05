use super::{inspector::Snapshot, models::*};
use crate::storage::{backups::models::FileEntry, error::Result};
use rusqlite::{Connection, OpenFlags, types::ValueRef};
use std::{
    collections::HashSet,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
pub const MAX_MESSAGES: u64 = 1_000_000;
pub struct Record {
    pub key: String,
    pub jid: String,
    pub title: String,
    pub sender: Option<String>,
    pub direction: &'static str,
    pub text: String,
    pub timestamp: Option<i64>,
    pub kind: i64,
    pub classification: &'static str,
    pub media: Option<Media>,
    pub reply: Option<String>,
}
pub struct Media {
    pub path: Option<String>,
    pub size: Option<u64>,
    pub hash: Option<String>,
    pub mime: Option<String>,
}
#[derive(Clone)]
pub enum Schema {
    Legacy,
    Modern,
}
impl Schema {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Legacy => "messages-v1",
            Self::Modern => "message-chat-jid-v1",
        }
    }
}
pub fn open(path: &Path, cancel: Arc<AtomicBool>) -> Result<Connection> {
    crate::storage::paths::open_regular_file(path)?;
    let mut c = sql(Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ))?;
    sql(c.execute_batch("PRAGMA query_only=ON; PRAGMA trusted_schema=OFF; PRAGMA temp_store=MEMORY; PRAGMA cache_size=-8192; PRAGMA cell_size_check=ON;"))?;
    sql(c.set_limit(rusqlite::limits::Limit::SQLITE_LIMIT_LENGTH, 1024 * 1024))?;
    let start = Instant::now();
    sql(c.progress_handler(
        10000,
        Some(move || cancel.load(Ordering::Acquire) || start.elapsed() > Duration::from_secs(600)),
    ))?;
    let quick: String = sql(c.query_row("PRAGMA quick_check(1)", [], |r| r.get(0)))?;
    if quick != "ok" {
        return Err(fail("native_database"));
    }
    Ok(c)
}
fn columns(c: &Connection, table: &str) -> Result<HashSet<String>> {
    let table_ok: i64 = sql(c.query_row(
        "SELECT count(*) FROM sqlite_schema WHERE type='table' AND rootpage>0 AND name=?1",
        [table],
        |r| r.get(0),
    ))?;
    if table_ok != 1 {
        return Ok(Default::default());
    }
    let mut s = sql(c.prepare("SELECT name FROM pragma_table_info(?1)"))?;
    let rows = sql(s.query_map([table], |r| r.get(0)))?;
    sql(rows.collect())
}
fn has(c: &Connection, t: &str, fields: &[&str]) -> Result<bool> {
    let cols = columns(c, t)?;
    Ok(fields.iter().all(|f| cols.contains(*f)))
}
pub fn schema(c: &Connection) -> Result<Schema> {
    if has(
        c,
        "message",
        &[
            "_id",
            "chat_row_id",
            "sender_jid_row_id",
            "from_me",
            "timestamp",
            "message_type",
            "text_data",
            "key_id",
        ],
    )? && has(c, "chat", &["_id", "jid_row_id", "subject"])?
        && has(c, "jid", &["_id", "raw_string"])?
    {
        return Ok(Schema::Modern);
    }
    if has(
        c,
        "messages",
        &[
            "_id",
            "key_remote_jid",
            "key_from_me",
            "key_id",
            "data",
            "timestamp",
            "remote_resource",
            "media_wa_type",
        ],
    )? {
        return Ok(Schema::Legacy);
    }
    Err(fail("native_schema"))
}
fn text(row: &rusqlite::Row<'_>, i: usize) -> rusqlite::Result<Option<String>> {
    match row.get_ref(i)? {
        ValueRef::Null => Ok(None),
        ValueRef::Text(b) => String::from_utf8(b.to_vec()).map(Some).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(i, rusqlite::types::Type::Text, Box::new(e))
        }),
        _ => Ok(None),
    }
}
fn file_hash(row: &rusqlite::Row<'_>, i: usize) -> rusqlite::Result<Option<String>> {
    match row.get_ref(i)? {
        ValueRef::Blob(b) if b.len() == 32 => {
            Ok(Some(b.iter().map(|b| format!("{b:02x}")).collect()))
        }
        ValueRef::Text(b) if b.len() == 64 && b.iter().all(|b| b.is_ascii_hexdigit()) => {
            Ok(Some(String::from_utf8_lossy(b).to_ascii_lowercase()))
        }
        _ => Ok(None),
    }
}
pub fn walk(
    c: &Connection,
    schema: &Schema,
    mut visit: impl FnMut(Record) -> Result<()>,
    mut check: impl FnMut(u64) -> Result<()>,
) -> Result<Counts> {
    let table = match schema {
        Schema::Legacy => "messages",
        Schema::Modern => "message",
    };
    // Sort only integer IDs/timestamps, not message bodies; fetch one bounded row at a time.
    let mut ids = sql(c.prepare(&format!(
        "SELECT _id FROM {table} ORDER BY timestamp,_id LIMIT 1000001"
    )))?;
    let mut rows = sql(ids.query([]))?;
    let media = matches!(schema, Schema::Modern)
        && has(
            c,
            "message_media",
            &[
                "message_row_id",
                "file_path",
                "file_size",
                "file_hash",
                "mime_type",
            ],
        )?;
    let quote = matches!(schema, Schema::Modern)
        && has(c, "message_quoted", &["message_row_id", "key_id"])?;
    let query=match schema{
        Schema::Legacy=>"SELECT m._id,m.key_id,m.key_remote_jid,NULL,m.remote_resource,m.key_from_me,m.data,m.timestamp,CAST(m.media_wa_type AS INTEGER),NULL,NULL,NULL,NULL,NULL FROM messages m WHERE m._id=?1".to_owned(),
        Schema::Modern=>format!("SELECT m._id,m.key_id,j.raw_string,c.subject,s.raw_string,m.from_me,m.text_data,m.timestamp,m.message_type,{},{}, {},{},{} FROM message m JOIN chat c ON c._id=m.chat_row_id JOIN jid j ON j._id=c.jid_row_id LEFT JOIN jid s ON s._id=m.sender_jid_row_id WHERE m._id=?1",
            if media{"(SELECT min(file_path) FROM message_media WHERE message_row_id=m._id HAVING count(*)=1)"}else{"NULL"},
            if media{"(SELECT min(file_size) FROM message_media WHERE message_row_id=m._id HAVING count(*)=1)"}else{"NULL"},
            if media{"(SELECT min(file_hash) FROM message_media WHERE message_row_id=m._id HAVING count(*)=1)"}else{"NULL"},
            if media{"(SELECT min(mime_type) FROM message_media WHERE message_row_id=m._id HAVING count(*)=1)"}else{"NULL"},
            if quote{"(SELECT min(key_id) FROM message_quoted WHERE message_row_id=m._id HAVING count(*)=1)"}else{"NULL"}),
    };
    let mut get = sql(c.prepare(&query))?;
    let mut counts = Counts::default();
    let mut chats = HashSet::new();
    while let Some(row) = sql(rows.next())? {
        check(counts.messages)?;
        if counts.messages >= MAX_MESSAGES {
            return Err(fail("native_limit"));
        }
        let source_id: i64 = sql(row.get(0))?;
        let mut values = sql(get.query([source_id]))?;
        let Some(r) = sql(values.next())? else {
            counts.unsupported += 1;
            continue;
        };
        let jid = sql(text(r, 2))?
            .filter(|s| !s.is_empty() && s.len() <= 256)
            .ok_or_else(|| fail("native_schema"))?;
        let from: Option<i64> = sql(r.get(5))?;
        let kind: Option<i64> = sql(r.get(8))?;
        let kind = kind.unwrap_or(-1);
        let classification = match kind {
            0 | 1 | 2 | 3 | 9 | 13 | 20 => "message",
            15 if matches!(schema, Schema::Modern) => "deleted",
            7 if matches!(schema, Schema::Modern) => "system",
            _ => {
                counts.unsupported += 1;
                "unknown"
            }
        };
        let media_kind = matches!(kind, 1 | 2 | 3 | 9 | 13 | 20);
        let body = sql(text(r, 6))?.unwrap_or_default();
        if body.len() > 1024 * 1024 {
            return Err(fail("native_limit"));
        }
        let sender = if from == Some(1) {
            Some("You".into())
        } else {
            sql(text(r, 4))?.or_else(|| (!jid.ends_with("@g.us")).then(|| jid.clone()))
        };
        let title = if jid.ends_with("@g.us") {
            sql(text(r, 3))?
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| jid.clone())
        } else {
            jid.clone()
        };
        let size: Option<i64> = sql(r.get(10))?;
        let record = Record {
            key: sql(text(r, 1))?.unwrap_or_else(|| source_id.to_string()),
            jid: jid.clone(),
            title,
            sender,
            direction: match from {
                Some(1) => "outgoing",
                Some(0) => "incoming",
                _ => "unknown",
            },
            text: body,
            timestamp: sql(r.get(7))?,
            kind,
            classification,
            reply: sql(text(r, 13))?,
            media: media_kind.then(|| Media {
                path: text(r, 9).ok().flatten(),
                size: size.filter(|s| *s >= 0).map(|s| s as u64),
                hash: file_hash(r, 11).ok().flatten(),
                mime: text(r, 12).ok().flatten(),
            }),
        };
        visit(record)?;
        chats.insert(jid);
        if chats.len() > 100_000 {
            return Err(fail("native_limit"));
        }
        counts.messages += 1;
    }
    check(counts.messages)?;
    counts.conversations = chats.len() as u64;
    if counts.messages == 0 {
        return Err(fail("native_schema"));
    }
    Ok(counts)
}
pub fn match_media<'a>(snapshot: &'a Snapshot, media: &Media) -> Option<&'a FileEntry> {
    let path = media.path.as_deref()?.replace('\\', "/");
    if path.split('/').any(|s| s == ".." || s == ".") {
        return None;
    }
    let marker = format!("{}/Media/", snapshot.manifest.source.base());
    let logical = if let Some((_, tail)) = path.split_once(&marker) {
        format!("Media/{tail}")
    } else if path.starts_with("Media/") {
        path
    } else {
        return None;
    };
    let hash = media.hash.as_deref()?;
    let size = media.size?;
    let matches: Vec<_> = snapshot
        .entries
        .iter()
        .filter(|f| {
            f.source_relative
                .split_once(&marker)
                .is_some_and(|(_, s)| format!("Media/{s}") == logical)
                && f.actual_bytes == size
                && f.sha256.as_deref() == Some(hash)
                && f.state == "verified"
        })
        .collect();
    (matches.len() == 1).then(|| matches[0])
}
