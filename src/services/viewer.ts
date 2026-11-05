import { convertFileSrc, invoke, isTauri } from '@tauri-apps/api/core';
import type { StorageFailure } from './storage';
export interface Conversation {
  id: string;
  archiveId: string;
  title: string;
  originalFilename: string | null;
  importedAt: number;
  messageCount: number;
  attachmentCount: number;
  mediaCount: number;
  lastPreview: string | null;
  lastLocalTimestamp: string | null;
  lastOriginalTimestamp: string | null;
  participants: string[];
  dateOrder: string | null;
}
export interface ConversationPage {
  items: Conversation[];
  total: number;
}
export interface Attachment {
  id: string;
  mediaId: string | null;
  filename: string;
  mediaType: string | null;
  sizeBytes: number | null;
  missing: boolean;
}
export interface SavedMessage {
  nativeDirection?: string | null;
  nativeTimestampMs?: number | null;
  nativeType?: number | null;
  nativeReplyKey?: string | null;
  nativeReplyId?: string | null;
  id: string;
  sequence: number;
  sender: string | null;
  text: string;
  originalTimestamp: string | null;
  localTimestamp: string | null;
  classification: string;
  attachments: Attachment[];
}
export interface MessagePage {
  items: SavedMessage[];
  hasOlder: boolean;
  hasNewer: boolean;
  anchorId: string | null;
}
export interface SearchHit {
  messageId: string;
  conversationId: string;
  conversationTitle: string;
  archiveId: string;
  sender: string | null;
  snippet: string;
  originalTimestamp: string | null;
  localTimestamp: string | null;
}
export interface SearchPage {
  items: SearchHit[];
  nextCursor: string | null;
  mode: 'trigram' | 'bounded-scan';
  scanned: number;
}
export interface MediaMetadata {
  id: string;
  filename: string;
  sizeBytes: number;
  mime: string;
  kind: 'image' | 'video' | 'audio' | 'document';
  width: number | null;
  height: number | null;
}
export interface MediaAccess {
  token: string;
  metadata: MediaMetadata;
}
export interface MediaPage {
  items: Attachment[];
  nextOffset: number | null;
}
const safeCodes = new Set([
  'viewer_failed',
  'invalid_request',
  'invalid_filter',
  'invalid_search',
  'conversation_missing',
  'message_missing',
  'viewer_limit',
  'media_missing',
  'media_denied',
  'media_unsupported',
  'media_limit',
  'media_changed',
  'media_document',
  'media_expired',
  'document_open_failed',
  'database_unavailable',
  'vault_unavailable',
  'unsafe_path',
  'runtime_unavailable',
]);
export function viewerFailure(error: unknown): StorageFailure {
  if (
    error &&
    typeof error === 'object' &&
    'code' in error &&
    typeof error.code === 'string' &&
    safeCodes.has(error.code) &&
    'message' in error &&
    typeof error.message === 'string'
  )
    return { code: error.code, message: error.message };
  return {
    code: 'viewer_failed',
    message: 'The local archive could not be read. Check your vault and retry.',
  };
}
async function call<T>(
  command: string,
  args: Record<string, unknown>,
): Promise<T> {
  if (!isTauri())
    throw {
      code: 'runtime_unavailable',
      message:
        'The chat reader requires ChatVault Desktop. Local archives cannot be opened in a browser preview.',
    };
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw viewerFailure(error);
  }
}
export const viewerService = {
  conversations: (
    query = '',
    archiveId: string | null = null,
    offset = 0,
    limit = 30,
  ) =>
    call<ConversationPage>('list_conversations', {
      query,
      archiveId,
      offset,
      limit,
    }),
  conversation: (conversationId: string) =>
    call<Conversation>('get_conversation', { conversationId }),
  messages: (
    conversationId: string,
    cursor: string | null = null,
    direction: 'older' | 'newer' | 'latest' = 'latest',
    limit = 50,
  ) =>
    call<MessagePage>('read_messages', {
      conversationId,
      cursor,
      direction,
      limit,
    }),
  context: (conversationId: string, messageId: string) =>
    call<MessagePage>('get_message_context', { conversationId, messageId }),
  search: (
    query: string,
    conversationId: string | null = null,
    cursor: string | null = null,
  ) => call<SearchPage>('search_messages', { query, conversationId, cursor }),
  media: (conversationId: string, offset = 0) =>
    call<MediaPage>('list_conversation_media', { conversationId, offset }),
  metadata: (mediaId: string) =>
    call<MediaMetadata>('get_media_metadata', { mediaId }),
  authorize: (mediaId: string) =>
    call<MediaAccess>('authorize_media', { mediaId }),
  revoke: (token: string) => call<void>('revoke_media', { token }),
  openDocument: (mediaId: string) => call<void>('open_document', { mediaId }),
};
export function privateMediaUrl(token: string) {
  return convertFileSrc(token, 'chatvault-media');
}
export function messageTime(message: {
  localTimestamp: string | null;
  originalTimestamp: string | null;
  nativeTimestampMs?: number | null;
}) {
  if (message.nativeTimestampMs != null) {
    const date = new Date(message.nativeTimestampMs);
    if (!Number.isNaN(date.valueOf()))
      return date.toISOString().replace('T', ' ') + ' (UTC)';
  }
  return message.localTimestamp
    ? `${message.localTimestamp.replace('T', ' ')} · local time`
    : (message.originalTimestamp ?? 'Timestamp unavailable');
}
export function messageDay(message: {
  localTimestamp: string | null;
  originalTimestamp: string | null;
  nativeTimestampMs?: number | null;
}) {
  if (message.nativeTimestampMs != null) {
    const date = new Date(message.nativeTimestampMs);
    if (!Number.isNaN(date.valueOf()))
      return date.toISOString().slice(0, 10) + ' (UTC)';
  }
  return (
    message.localTimestamp?.slice(0, 10) ??
    (message.originalTimestamp
      ? `Original date · ${message.originalTimestamp.split(',')[0]}`
      : 'Date unknown')
  );
}
