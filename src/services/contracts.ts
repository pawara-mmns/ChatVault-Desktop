import type {
  Archive,
  BackupSnapshot,
  Conversation,
  Device,
  Message,
} from '../domain/models';
/** Reserved interfaces, deliberately without fake adapters or success responses. */
export interface DeviceService {
  listConnected(): Promise<readonly Device[]>;
}
export interface BackupService {
  listSnapshots(): Promise<readonly BackupSnapshot[]>;
  createSnapshot(deviceId: string): Promise<BackupSnapshot>;
}
export interface ArchiveService {
  listArchives(query?: string): Promise<readonly Archive[]>;
  listConversations(archiveId: string): Promise<readonly Conversation[]>;
  listMessages(conversationId: string): Promise<readonly Message[]>;
}
export interface ImportService {
  /** A future platform file-picker token, not an unrestricted file path. */
  importChatExport(selectionToken: string): Promise<Archive>;
}
