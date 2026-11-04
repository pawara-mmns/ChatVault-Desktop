import type { BackupSnapshot, Device } from '../domain/models';
export type { ArchiveService, ImportService } from './imports';

/** Device/backup operations remain reserved for later steps; no fake adapters exist. */
export interface DeviceService {
  listConnected(): Promise<readonly Device[]>;
}
export interface BackupService {
  listSnapshots(): Promise<readonly BackupSnapshot[]>;
  createSnapshot(deviceId: string): Promise<BackupSnapshot>;
}
