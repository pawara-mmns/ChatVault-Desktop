use super::{
    models::{ParsedChat, diagnostic},
    zip_processor::MediaFile,
};
use std::collections::{HashMap, HashSet};
use unicode_normalization::UnicodeNormalization;
pub struct AttachmentLink {
    pub message_index: usize,
    pub original_name: Option<String>,
    pub media_index: Option<usize>,
}
pub fn match_references(chat: &mut ParsedChat, media: &[MediaFile]) -> Vec<AttachmentLink> {
    let mut links = Vec::new();
    let key = |name: &str| name.nfc().collect::<String>().to_lowercase();
    let mut exact = HashMap::new();
    let mut basenames = HashMap::new();
    let mut used = HashSet::new();
    for (index, file) in media.iter().enumerate() {
        exact.insert(key(&file.original_name), index);
        basenames
            .entry(key(file.original_name.rsplit('/').next().unwrap_or("")))
            .and_modify(|value| *value = None)
            .or_insert(Some(index));
    }
    for (message_index, message) in chat.messages.iter().enumerate() {
        for reference in &message.media {
            let matched = reference.as_ref().and_then(|reference| {
                if let Some(index) = exact.get(&key(reference)) {
                    return Some(*index);
                }
                let base = reference.rsplit('/').next().unwrap_or(reference);
                basenames.get(&key(base)).copied().flatten()
            });
            if matched.is_none() {
                diagnostic(
                    &mut chat.diagnostics,
                    "missing_attachments",
                    "Referenced media was omitted, missing, or ambiguous. The message/reference is retained.",
                    Some(message.line_number),
                );
            }
            if let Some(index) = matched {
                used.insert(index);
            }
            links.push(AttachmentLink {
                message_index,
                original_name: reference.clone(),
                media_index: matched,
            });
        }
    }
    if used.len() < media.len() {
        diagnostic(
            &mut chat.diagnostics,
            "unmatched_media",
            "Some extracted files could not be linked to a message. They are retained with archive media metadata.",
            None,
        );
    }
    links
}
