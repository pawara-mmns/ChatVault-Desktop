use super::{Result, models::*};
use crate::storage::{StorageState, error::StorageError};
use tauri::State;
async fn worker<T: Send + 'static>(
    state: StorageState,
    job: impl FnOnce(&mut crate::storage::service::StorageService) -> Result<T> + Send + 'static,
) -> Result<T> {
    tauri::async_runtime::spawn_blocking(move || state.run(false, job))
        .await
        .map_err(|_| {
            StorageError::new(
                "viewer_failed",
                "The local archive request could not finish. Retry or restart ChatVault.",
            )
        })?
}
#[tauri::command]
pub async fn list_conversations(
    state: State<'_, StorageState>,
    query: String,
    archive_id: Option<String>,
    offset: u32,
    limit: u32,
) -> Result<ConversationPage> {
    worker(state.inner().clone(), move |s| {
        s.list_conversations(&query, archive_id.as_deref(), offset, limit)
    })
    .await
}
#[tauri::command]
pub async fn get_conversation(
    state: State<'_, StorageState>,
    conversation_id: String,
) -> Result<Conversation> {
    worker(state.inner().clone(), move |s| {
        s.conversation(&conversation_id)
    })
    .await
}
#[tauri::command]
pub async fn read_messages(
    state: State<'_, StorageState>,
    conversation_id: String,
    cursor: Option<String>,
    direction: Direction,
    limit: u32,
) -> Result<MessagePage> {
    worker(state.inner().clone(), move |s| {
        s.read_messages(&conversation_id, cursor.as_deref(), direction, limit)
    })
    .await
}
#[tauri::command]
pub async fn get_message_context(
    state: State<'_, StorageState>,
    conversation_id: String,
    message_id: String,
) -> Result<MessagePage> {
    worker(state.inner().clone(), move |s| {
        s.message_context(&conversation_id, &message_id)
    })
    .await
}
#[tauri::command]
pub async fn search_messages(
    state: State<'_, StorageState>,
    query: String,
    conversation_id: Option<String>,
    cursor: Option<String>,
) -> Result<SearchPage> {
    worker(state.inner().clone(), move |s| {
        s.search_messages(&query, conversation_id.as_deref(), cursor.as_deref())
    })
    .await
}
#[tauri::command]
pub async fn list_conversation_media(
    state: State<'_, StorageState>,
    conversation_id: String,
    offset: u32,
) -> Result<MediaPage> {
    worker(state.inner().clone(), move |s| {
        s.conversation_media(&conversation_id, offset)
    })
    .await
}
#[tauri::command]
pub async fn get_media_metadata(
    state: State<'_, StorageState>,
    media_id: String,
) -> Result<MediaMetadata> {
    worker(state.inner().clone(), move |s| s.media_metadata(&media_id)).await
}
#[tauri::command]
pub async fn authorize_media(
    state: State<'_, StorageState>,
    media_id: String,
) -> Result<MediaAccess> {
    worker(state.inner().clone(), move |s| s.authorize_media(&media_id)).await
}
#[tauri::command]
pub async fn revoke_media(state: State<'_, StorageState>, token: String) -> Result<()> {
    worker(state.inner().clone(), move |s| s.revoke_media(&token)).await
}
#[tauri::command]
pub async fn open_document(state: State<'_, StorageState>, media_id: String) -> Result<()> {
    worker(state.inner().clone(), move |s| s.open_document(&media_id)).await
}
