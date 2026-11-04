/** UI-facing domain contracts. SQL timestamps/rows are mapped at repository boundaries. */
export type EntityId = string;
export type ISODateTime = string;
export type { ConnectedDevice as Device } from '../services/devices';
export interface BackupSnapshot {
  id: EntityId;
  deviceId: EntityId;
  createdAt: ISODateTime;
  status: 'in-progress' | 'complete' | 'failed';
  accessibleFileCount: number;
  sizeBytes: number;
}
export interface Archive {
  id: EntityId;
  title: string;
  source: 'chat-export';
  importedAt: ISODateTime;
  conversationIds: EntityId[];
}
export interface Conversation {
  id: EntityId;
  archiveId: EntityId;
  title: string;
  participants: string[];
}
export interface Message {
  id: EntityId;
  conversationId: EntityId;
  sender: string | null;
  sentAt: ISODateTime | null;
  originalTimestamp: string | null;
  localTimestamp: string | null;
  timezoneStatus: 'unknown';
  classification: 'message' | 'system' | 'unknown' | 'deleted';
  text: string;
  attachmentIds: EntityId[];
}
export interface Attachment {
  id: EntityId;
  messageId: EntityId;
  fileName: string | null;
  mimeType: string | null;
  sizeBytes: number | null;
  /** Future scoped content reference, never an arbitrary frontend filesystem path. */
  contentReference: string | null;
  isMissing: boolean;
}
