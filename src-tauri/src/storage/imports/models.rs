use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum DateOrder {
    #[default]
    Auto,
    DayFirst,
    MonthFirst,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub code: String,
    pub severity: String,
    pub description: String,
    pub occurrence_count: u32,
    pub line_number: Option<u32>,
}
pub fn diagnostic(items: &mut Vec<Diagnostic>, code: &str, description: &str, line: Option<u32>) {
    if let Some(existing) = items.iter_mut().find(|item| item.code == code) {
        existing.occurrence_count += 1;
    } else {
        items.push(Diagnostic {
            code: code.into(),
            severity: "warning".into(),
            description: description.into(),
            occurrence_count: 1,
            line_number: line,
        });
    }
}

#[derive(Clone)]
pub struct ParsedMessage {
    pub sender: Option<String>,
    pub text: String,
    pub original_timestamp: Option<String>,
    pub local_timestamp: Option<String>,
    pub classification: String,
    pub original_record: String,
    pub media: Vec<Option<String>>,
    pub line_number: u32,
}
pub struct ParsedChat {
    pub messages: Vec<ParsedMessage>,
    pub participants: Vec<String>,
    pub diagnostics: Vec<Diagnostic>,
    pub date_order_required: bool,
    pub encoding: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectedImport {
    pub token: String,
    pub filename: String,
    pub size_bytes: u64,
    pub source_format: String,
    pub duplicate_archive: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageSample {
    pub sender: Option<String>,
    pub text: String,
    pub original_timestamp: Option<String>,
    pub local_timestamp: Option<String>,
    pub classification: String,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub selection: SelectedImport,
    pub title: String,
    pub message_count: usize,
    pub participant_count: usize,
    pub participants: Vec<String>,
    pub media_count: usize,
    pub attachment_count: usize,
    pub missing_attachment_count: usize,
    pub date_order_required: bool,
    pub date_order: DateOrder,
    pub encoding: String,
    pub samples: Vec<MessageSample>,
    pub diagnostics: Vec<Diagnostic>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveSummary {
    pub id: String,
    pub title: String,
    pub original_filename: Option<String>,
    pub source_format: Option<String>,
    pub imported_at: i64,
    pub message_count: i64,
    pub media_count: i64,
    pub missing_media_count: i64,
    pub total_bytes: i64,
    pub import_status: String,
    pub original_sha256: Option<String>,
    pub date_order: Option<String>,
    pub text_encoding: Option<String>,
    pub diagnostics: Vec<Diagnostic>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchivePage {
    pub items: Vec<ArchiveSummary>,
    pub total: i64,
    pub offset: u32,
    pub limit: u32,
}

#[derive(Clone)]
pub struct ImportLimits {
    pub source_bytes: u64,
    pub text_bytes: u64,
    pub entry_bytes: u64,
    pub expanded_bytes: u64,
    pub entries: usize,
    pub compression_ratio: u64,
    pub messages: usize,
}
impl Default for ImportLimits {
    fn default() -> Self {
        Self {
            source_bytes: 256 * 1024 * 1024,
            text_bytes: 16 * 1024 * 1024,
            entry_bytes: 64 * 1024 * 1024,
            expanded_bytes: 512 * 1024 * 1024,
            entries: 5000,
            compression_ratio: 200,
            messages: 100_000,
        }
    }
}
