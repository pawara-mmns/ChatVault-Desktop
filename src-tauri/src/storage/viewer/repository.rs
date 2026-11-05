use super::{Result, failure, id, models::*, search::fold};
use crate::storage::{error::StorageError, service::StorageService};
use rusqlite::{OptionalExtension, params};
const CONVERSATIONS: &str = "SELECT c.id,c.archive_id,c.title,a.original_filename,a.created_at,(SELECT count(*) FROM messages WHERE conversation_id=c.id),(SELECT count(*) FROM attachments t JOIN messages m ON m.id=t.message_id WHERE m.conversation_id=c.id),(SELECT count(*) FROM archive_media WHERE archive_id=a.id),(SELECT substr(message_text,1,140) FROM messages WHERE conversation_id=c.id ORDER BY sequence_number DESC,id DESC LIMIT 1),(SELECT local_timestamp FROM messages WHERE conversation_id=c.id ORDER BY sequence_number DESC,id DESC LIMIT 1),(SELECT original_timestamp FROM messages WHERE conversation_id=c.id ORDER BY sequence_number DESC,id DESC LIMIT 1),c.participants_json,a.date_order FROM conversations c JOIN archives a ON a.id=c.archive_id";
fn conversation_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Conversation> {
    let participants: String = row.get(11)?;
    Ok(Conversation {
        id: row.get(0)?,
        archive_id: row.get(1)?,
        title: row.get(2)?,
        original_filename: row.get(3)?,
        imported_at: row.get(4)?,
        message_count: row.get(5)?,
        attachment_count: row.get(6)?,
        media_count: row.get(7)?,
        last_preview: row.get(8)?,
        last_local_timestamp: row.get(9)?,
        last_original_timestamp: row.get(10)?,
        participants: serde_json::from_str(&participants).unwrap_or_default(),
        date_order: row.get(12)?,
    })
}
impl StorageService {
    pub fn list_conversations(
        &self,
        query: &str,
        archive: Option<&str>,
        offset: u32,
        limit: u32,
    ) -> Result<ConversationPage> {
        if query.chars().count() > 200 || offset > 1_000_000 {
            return Err(failure(
                "invalid_filter",
                "The conversation filter is too long.",
            ));
        }
        if let Some(value) = archive {
            id(value)?;
        }
        let query = fold(query.trim());
        let predicate =
            " WHERE instr(chatvault_fold(c.title),?1)>0 AND (?2 IS NULL OR c.archive_id=?2)";
        let total = self
            .connection
            .query_row(
                &format!("SELECT count(*) FROM conversations c{predicate}"),
                params![query, archive],
                |r| r.get(0),
            )
            .map_err(|_| StorageError::database())?;
        let mut statement=self.connection.prepare(&format!("{CONVERSATIONS}{predicate} ORDER BY 10 DESC,a.created_at DESC,c.id LIMIT ?3 OFFSET ?4")).map_err(|_|StorageError::database())?;
        let items = statement
            .query_map(
                params![query, archive, limit.clamp(1, 50), offset],
                conversation_row,
            )
            .map_err(|_| StorageError::database())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|_| StorageError::database())?;
        Ok(ConversationPage { items, total })
    }
    pub fn conversation(&self, conversation_id: &str) -> Result<Conversation> {
        id(conversation_id)?;
        self.connection
            .query_row(
                &format!("{CONVERSATIONS} WHERE c.id=?1"),
                [conversation_id],
                conversation_row,
            )
            .optional()
            .map_err(|_| StorageError::database())?
            .ok_or_else(|| {
                failure(
                    "conversation_missing",
                    "This conversation is no longer available in the local database.",
                )
            })
    }
    fn message_cursor(&self, conversation_id: &str, message_id: &str) -> Result<i64> {
        id(message_id)?;
        self.connection
            .query_row(
                "SELECT sequence_number FROM messages WHERE id=?1 AND conversation_id=?2",
                params![message_id, conversation_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(|_| StorageError::database())?
            .ok_or_else(|| {
                failure(
                    "message_missing",
                    "The requested message is not part of this conversation.",
                )
            })
    }
    pub fn read_messages(
        &self,
        conversation_id: &str,
        cursor: Option<&str>,
        direction: Direction,
        limit: u32,
    ) -> Result<MessagePage> {
        self.conversation(conversation_id)?;
        let limit = limit.clamp(1, 60);
        let sequence = cursor
            .map(|value| self.message_cursor(conversation_id, value))
            .transpose()?;
        let (predicate, order) = match direction {
            Direction::Latest => ("1=1", "DESC"),
            Direction::Older if sequence.is_some() => ("sequence_number<?2", "DESC"),
            Direction::Newer if sequence.is_some() => ("sequence_number>?2", "ASC"),
            _ => {
                return Err(failure(
                    "invalid_request",
                    "Choose a valid message cursor for this page.",
                ));
            }
        };
        let mut statement=self.connection.prepare(&format!("SELECT id,sequence_number,sender,message_text,original_timestamp,local_timestamp,classification,native_direction,native_timestamp_ms,native_type,native_reply_key,native_reply_id FROM messages WHERE conversation_id=?1 AND {predicate} AND ?2 IS ?2 ORDER BY sequence_number {order},id {order} LIMIT ?3")).map_err(|_|StorageError::database())?;
        let mut items = statement
            .query_map(params![conversation_id, sequence, limit], |row| {
                Ok(Message {
                    id: row.get(0)?,
                    sequence: row.get(1)?,
                    sender: row.get(2)?,
                    text: row.get(3)?,
                    original_timestamp: row.get(4)?,
                    local_timestamp: row.get(5)?,
                    classification: row.get(6)?,
                    native_direction: row.get(7)?,
                    native_timestamp_ms: row.get(8)?,
                    native_type: row.get(9)?,
                    native_reply_key: row.get(10)?,
                    native_reply_id: row.get(11)?,
                    attachments: Vec::new(),
                })
            })
            .map_err(|_| StorageError::database())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|_| StorageError::database())?;
        if order == "DESC" {
            items.reverse();
        }
        let mut attachments=self.connection.prepare("SELECT t.id,t.archive_media_id,COALESCE(t.original_filename,'Attachment'),t.media_type,t.file_size,t.is_missing FROM attachments t WHERE message_id=?1 ORDER BY t.id LIMIT 1001").map_err(|_|StorageError::database())?;
        for message in &mut items {
            message.attachments = attachments
                .query_map([&message.id], |r| {
                    Ok(Attachment {
                        id: r.get(0)?,
                        media_id: r.get(1)?,
                        filename: r.get(2)?,
                        media_type: r.get(3)?,
                        size_bytes: r.get(4)?,
                        missing: r.get(5)?,
                    })
                })
                .map_err(|_| StorageError::database())?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|_| StorageError::database())?;
            if message.attachments.len() > 1000 {
                return Err(failure(
                    "viewer_limit",
                    "A message contains more attachments than this viewer supports.",
                ));
            }
        }
        let first = items.first().map(|m| m.sequence).or(sequence);
        let last = items.last().map(|m| m.sequence).or(sequence);
        let exists = |comparison: &str, seq: Option<i64>| -> Result<bool> {
            self.connection.query_row(&format!("SELECT EXISTS(SELECT 1 FROM messages WHERE conversation_id=?1 AND sequence_number{comparison}?2)"),params![conversation_id,seq],|r|r.get(0)).map_err(|_|StorageError::database())
        };
        Ok(MessagePage {
            has_older: exists("<", first)?,
            has_newer: exists(">", last)?,
            items,
            anchor_id: None,
        })
    }
    pub fn message_context(&self, conversation_id: &str, message_id: &str) -> Result<MessagePage> {
        self.message_cursor(conversation_id, message_id)?;
        let mut before =
            self.read_messages(conversation_id, Some(message_id), Direction::Older, 25)?;
        let after = self.read_messages(conversation_id, Some(message_id), Direction::Newer, 25)?;
        // Read the anchor with an inclusive indexed query; never infer IDs or dates.
        let previous = before.items.last().map(|m| m.id.as_str());
        let anchor = if let Some(previous) = previous {
            self.read_messages(conversation_id, Some(previous), Direction::Newer, 1)?
        } else {
            let mut page =
                self.read_messages(conversation_id, Some(message_id), Direction::Older, 1)?;
            let row=self.connection.query_row("SELECT id,sequence_number,sender,message_text,original_timestamp,local_timestamp,classification,native_direction,native_timestamp_ms,native_type,native_reply_key,native_reply_id FROM messages WHERE id=?1 AND conversation_id=?2",params![message_id,conversation_id],|r|Ok(Message{id:r.get(0)?,sequence:r.get(1)?,sender:r.get(2)?,text:r.get(3)?,original_timestamp:r.get(4)?,local_timestamp:r.get(5)?,classification:r.get(6)?,native_direction:r.get(7)?,native_timestamp_ms:r.get(8)?,native_type:r.get(9)?,native_reply_key:r.get(10)?,native_reply_id:r.get(11)?,attachments:Vec::new()})).map_err(|_|StorageError::database())?;
            page.items.push(row);
            page
        };
        before.items.extend(anchor.items);
        before.items.extend(after.items);
        before.has_newer = after.has_newer;
        before.anchor_id = Some(message_id.to_owned());
        // Populate attachments on a first-message anchor too, using the same read path.
        if let Some(anchor) = before.items.iter_mut().find(|m| m.id == message_id) {
            let mut statement=self.connection.prepare("SELECT id,archive_media_id,COALESCE(original_filename,'Attachment'),media_type,file_size,is_missing FROM attachments WHERE message_id=?1 ORDER BY id LIMIT 1000").map_err(|_|StorageError::database())?;
            anchor.attachments = statement
                .query_map([message_id], |r| {
                    Ok(Attachment {
                        id: r.get(0)?,
                        media_id: r.get(1)?,
                        filename: r.get(2)?,
                        media_type: r.get(3)?,
                        size_bytes: r.get(4)?,
                        missing: r.get(5)?,
                    })
                })
                .map_err(|_| StorageError::database())?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|_| StorageError::database())?;
        }
        Ok(before)
    }
    pub fn conversation_media(&self, conversation_id: &str, offset: u32) -> Result<MediaPage> {
        let conversation = self.conversation(conversation_id)?;
        if offset > 100_000 {
            return Err(failure("invalid_request", "The media page is invalid."));
        }
        let mut statement=self.connection.prepare("SELECT id,original_filename,media_type,file_size FROM archive_media WHERE archive_id=?1 ORDER BY id LIMIT 51 OFFSET ?2").map_err(|_|StorageError::database())?;
        let mut items = statement
            .query_map(params![conversation.archive_id, offset], |r| {
                let id: String = r.get(0)?;
                Ok(Attachment {
                    id: id.clone(),
                    media_id: Some(id),
                    filename: r.get(1)?,
                    media_type: r.get(2)?,
                    size_bytes: r.get(3)?,
                    missing: false,
                })
            })
            .map_err(|_| StorageError::database())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|_| StorageError::database())?;
        let more = items.len() > 50;
        items.truncate(50);
        Ok(MediaPage {
            items,
            next_offset: more.then_some(offset + 50),
        })
    }
}
