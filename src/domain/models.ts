/** Domain contracts only. No database or content persistence exists in Step 01. */
export type EntityId = string;
export type ISODateTime = string;
export interface Device {
  id: EntityId;
  displayName: string;
  platform: 'android';
  connection: 'usb';
  status: 'connected' | 'disconnected' | 'unauthorized';
}
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
  sender: string;
  sentAt: ISODateTime | null;
  text: string;
  attachmentIds: EntityId[];
}
export interface Attachment {
  id: EntityId;
  messageId: EntityId;
  fileName: string;
  mimeType: string | null;
  sizeBytes: number | null;
  /** Future scoped content reference, never an arbitrary frontend filesystem path. */
  contentReference: string;
}
