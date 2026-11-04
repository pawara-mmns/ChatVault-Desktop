use super::{Result, failure, models::*};
use crate::storage::service::StorageService;
use rusqlite::{Connection, functions::FunctionFlags, params};
use unicode_normalization::UnicodeNormalization;
pub fn fold(value: &str) -> String {
    value
        .nfc()
        .flat_map(char::to_lowercase)
        .collect::<String>()
        .nfc()
        .collect()
}
pub fn register(connection: &Connection) -> Result<()> {
    connection
        .create_scalar_function(
            "chatvault_fold",
            1,
            FunctionFlags::SQLITE_UTF8
                | FunctionFlags::SQLITE_DETERMINISTIC
                | FunctionFlags::SQLITE_INNOCUOUS,
            |ctx| Ok(fold(&ctx.get::<String>(0)?)),
        )
        .map_err(|_| crate::storage::error::StorageError::database())
}
pub(crate) fn index_conversation(connection: &Connection, conversation: &str) -> Result<()> {
    connection.execute("INSERT INTO message_search(rowid,body,sender) SELECT rowid,chatvault_fold(message_text),chatvault_fold(COALESCE(sender,'')) FROM messages WHERE conversation_id=?1",[conversation]).map_err(|_|crate::storage::error::StorageError::database())?;
    Ok(())
}
fn snippet(text: &str, query: &str) -> String {
    let folded = fold(text);
    let approximate = folded
        .find(query)
        .map(|i| folded[..i].chars().count())
        .unwrap_or(0);
    let start = approximate.saturating_sub(65);
    let mut output = String::new();
    if start > 0 {
        output.push('…');
    }
    output.extend(text.chars().skip(start).take(240));
    if text.chars().count() > start + 240 {
        output.push('…');
    }
    output
}
impl StorageService {
    pub fn search_messages(
        &self,
        query: &str,
        conversation: Option<&str>,
        cursor: Option<&str>,
    ) -> Result<SearchPage> {
        let query = fold(query.trim());
        if query.is_empty() || query.chars().count() > 200 || query.chars().any(char::is_control) {
            return Err(failure(
                "invalid_search",
                "Enter between 1 and 200 characters of message text or a sender name.",
            ));
        }
        if let Some(id) = conversation {
            self.conversation(id)?;
        }
        let after = match cursor {
            Some(s) => s
                .parse::<i64>()
                .ok()
                .filter(|v| *v >= 0)
                .ok_or_else(|| failure("invalid_request", "The search cursor is invalid."))?,
            None => 0,
        };
        let indexed = query.chars().count() >= 3;
        let sql = if indexed {
            "SELECT m.rowid,m.id,m.conversation_id,c.title,c.archive_id,m.sender,m.message_text,m.original_timestamp,m.local_timestamp FROM message_search f JOIN messages m ON m.rowid=f.rowid JOIN conversations c ON c.id=m.conversation_id WHERE message_search MATCH ?1 AND m.rowid>?2 AND (?3 IS NULL OR m.conversation_id=?3) ORDER BY m.rowid LIMIT 51"
        } else {
            "SELECT m.rowid,m.id,m.conversation_id,c.title,c.archive_id,m.sender,m.message_text,m.original_timestamp,m.local_timestamp FROM messages m JOIN conversations c ON c.id=m.conversation_id WHERE m.rowid>?2 AND (?3 IS NULL OR m.conversation_id=?3) AND ?1 IS NOT NULL ORDER BY m.rowid LIMIT 2001"
        };
        let phrase = format!("\"{}\"", query.replace('"', "\"\""));
        let mut statement = self
            .connection
            .prepare(sql)
            .map_err(|_| crate::storage::error::StorageError::database())?;
        let mut rows = statement
            .query(params![
                if indexed { &phrase } else { &query },
                after,
                conversation
            ])
            .map_err(|_| crate::storage::error::StorageError::database())?;
        let mut items = Vec::new();
        let mut scanned = 0;
        let mut last = after;
        let mut more = false;
        while let Some(row) = rows
            .next()
            .map_err(|_| crate::storage::error::StorageError::database())?
        {
            if items.len() == 50 || scanned == 2000 {
                more = true;
                break;
            }
            let row_id: i64 = row
                .get(0)
                .map_err(|_| crate::storage::error::StorageError::database())?;
            let sender: Option<String> = row
                .get(5)
                .map_err(|_| crate::storage::error::StorageError::database())?;
            let text: String = row
                .get(6)
                .map_err(|_| crate::storage::error::StorageError::database())?;
            scanned += 1;
            last = row_id;
            if indexed
                || fold(&text).contains(&query)
                || sender.as_ref().is_some_and(|s| fold(s).contains(&query))
            {
                items.push(SearchHit {
                    message_id: row
                        .get(1)
                        .map_err(|_| crate::storage::error::StorageError::database())?,
                    conversation_id: row
                        .get(2)
                        .map_err(|_| crate::storage::error::StorageError::database())?,
                    conversation_title: row
                        .get(3)
                        .map_err(|_| crate::storage::error::StorageError::database())?,
                    archive_id: row
                        .get(4)
                        .map_err(|_| crate::storage::error::StorageError::database())?,
                    sender,
                    snippet: snippet(&text, &query),
                    original_timestamp: row
                        .get(7)
                        .map_err(|_| crate::storage::error::StorageError::database())?,
                    local_timestamp: row
                        .get(8)
                        .map_err(|_| crate::storage::error::StorageError::database())?,
                });
            }
        }
        Ok(SearchPage {
            items,
            next_cursor: more.then(|| last.to_string()),
            mode: if indexed { "trigram" } else { "bounded-scan" },
            scanned,
        })
    }
}
