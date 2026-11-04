import type { BackupSnapshot } from '../domain/models';
export type { ArchiveService, ImportService } from './imports';

export type { DeviceService } from './devices';
/** Backup creation remains reserved for Step 06. */
export interface BackupService {
  listSnapshots(): Promise<readonly BackupSnapshot[]>;
  createSnapshot(deviceId: string): Promise<BackupSnapshot>;
}
