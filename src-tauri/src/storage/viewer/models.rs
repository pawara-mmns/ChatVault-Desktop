use serde::{Deserialize, Serialize};

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    pub id: String,
    pub archive_id: String,
    pub title: String,
    pub original_filename: Option<String>,
    pub imported_at: i64,
    pub message_count: i64,
    pub attachment_count: i64,
    pub media_count: i64,
    pub last_preview: Option<String>,
    pub last_local_timestamp: Option<String>,
    pub last_original_timestamp: Option<String>,
    pub participants: Vec<String>,
    pub date_order: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationPage {
    pub items: Vec<Conversation>,
    pub total: i64,
}
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    pub id: String,
    pub media_id: Option<String>,
    pub filename: String,
    pub media_type: Option<String>,
    pub size_bytes: Option<i64>,
    pub missing: bool,
}
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub native_direction: Option<String>,
    pub native_timestamp_ms: Option<i64>,
    pub native_type: Option<i64>,
    pub native_reply_key: Option<String>,
    pub native_reply_id: Option<String>,
    pub id: String,
    pub sequence: i64,
    pub sender: Option<String>,
    pub text: String,
    pub original_timestamp: Option<String>,
    pub local_timestamp: Option<String>,
    pub classification: String,
    pub attachments: Vec<Attachment>,
}
#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Older,
    Newer,
    Latest,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessagePage {
    pub items: Vec<Message>,
    pub has_older: bool,
    pub has_newer: bool,
    pub anchor_id: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub message_id: String,
    pub conversation_id: String,
    pub conversation_title: String,
    pub archive_id: String,
    pub sender: Option<String>,
    pub snippet: String,
    pub original_timestamp: Option<String>,
    pub local_timestamp: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchPage {
    pub items: Vec<SearchHit>,
    pub next_cursor: Option<String>,
    pub mode: &'static str,
    pub scanned: usize,
}
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MediaMetadata {
    pub id: String,
    pub filename: String,
    pub size_bytes: u64,
    pub mime: String,
    pub kind: &'static str,
    pub width: Option<u32>,
    pub height: Option<u32>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaAccess {
    pub token: String,
    pub metadata: MediaMetadata,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaPage {
    pub items: Vec<Attachment>,
    pub next_offset: Option<u32>,
}
