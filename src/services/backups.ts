import { invoke, isTauri } from '@tauri-apps/api/core';
export type Source = 'whatsapp' | 'business';
export type Category =
  | 'databases'
  | 'images'
  | 'videos'
  | 'documents'
  | 'audio'
  | 'voice_notes'
  | 'other';
export const categoryLabels: Record<Category, string> = {
  databases: 'Encrypted databases',
  images: 'Images',
  videos: 'Videos',
  documents: 'Documents',
  audio: 'Audio',
  voice_notes: 'Voice notes',
  other: 'Other discovered media (including stickers)',
};
export interface BackupFailure {
  code: string;
  message: string;
  diagnostic: string | null;
}
export interface BackupFile {
  id: string;
  category: Category;
  sourceRelative: string;
  localRelative: string;
  originalName: string;
  expectedSize: number | null;
  actualBytes: number;
  sha256: string | null;
  localVerification: string;
  sourceVerification: string;
  sizeValidation: string;
  state: string;
  errorCode: string | null;
}
export interface Plan {
  scannedCategories?: Category[];
  skippedEntries: number;
  inaccessibleLocations: number;
  id: string;
  deviceId: string;
  deviceName: string | null;
  source: Source;
  complete: boolean;
  warnings: string[];
  categories: {
    category: Category;
    files: number;
    knownBytes: number;
    unknownSizes: number;
  }[];
  totalFiles: number;
  knownBytes: number;
  unknownSizes: number;
  availableBytes: number | null;
  vaultPath: string;
}
export interface Selection {
  categories: Category[];
  fileIds: string[] | null;
  sourceReread: boolean;
  acknowledgeIncomplete: boolean;
  acknowledgeCapacity: boolean;
}
export interface ScanOptions {
  categories: Category[];
  maxObjects: number;
  maxFiles: number;
  maxDepth: number;
  timeoutSeconds: number;
}
export interface ScanReport {
  elapsedMs: number;
  directories?: number;
  objects: number;
  folders: number;
  files: number;
  inventoriedFiles: number;
  knownBytes: number;
  operation: string | null;
  operationPending: boolean;
  operationElapsedMs: number;
  lastSuccessfulOperation: string | null;
  successfulCalls: number;
  errors: number;
  errorCategory: string | null;
  diagnostic: string | null;
  slow: boolean;
  deadlineReached: boolean;
  timeoutSeconds: number;
  maxObjects: number;
  maxDepth: number;
}
export interface BackupProgress {
  operationId?: string | null;
  revision?: number;
  scan?: ScanReport | null;
  snapshotId: string | null;
  phase: string;
  source: Source | null;
  currentFilename: string | null;
  totalFiles: number;
  successfulFiles: number;
  failedFiles: number;
  copiedBytes: number;
  knownTotalBytes: number;
  unknownSizes: number;
  verifiedBytes: number;
  rateBytesPerSecond: number | null;
  cancelRequested: boolean;
  error: BackupFailure | null;
  updatedAt: number;
}
export interface Snapshot {
  id: string;
  sourceApp: Source | null;
  deviceName: string | null;
  phase: string;
  createdAt: number;
  completedAt: number | null;
  fileCount: number;
  successfulCount: number;
  failedCount: number;
  skippedCount: number;
  totalBytes: number;
  verifiedBytes: number;
  manifestPath: string | null;
  localIntegrity: string;
  localVerifiedAt: number | null;
  warnings: string[];
  destination: string;
}
export interface History {
  items: Snapshot[];
  total: number;
  completedCount: number;
  completedBytes: number;
}
export interface BackupService {
  available(): boolean;
  prepare(
    deviceId: string,
    source: Source,
    options?: ScanOptions,
  ): Promise<Plan>;
  planFiles(
    planId: string,
    category: Category | null,
    offset: number,
  ): Promise<BackupFile[]>;
  start(planId: string, selection: Selection): Promise<BackupProgress>;
  progress(): Promise<BackupProgress>;
  cancel(): Promise<void>;
  history(offset?: number): Promise<History>;
  files(snapshotId: string, offset: number): Promise<BackupFile[]>;
  verify(snapshotId: string): Promise<BackupProgress>;
  cleanup(snapshotId: string): Promise<void>;
  open(snapshotId: string): Promise<void>;
}
export const idleProgress: BackupProgress = {
  snapshotId: null,
  phase: 'idle',
  source: null,
  currentFilename: null,
  totalFiles: 0,
  successfulFiles: 0,
  failedFiles: 0,
  copiedBytes: 0,
  knownTotalBytes: 0,
  unknownSizes: 0,
  verifiedBytes: 0,
  rateBytesPerSecond: null,
  cancelRequested: false,
  error: null,
  updatedAt: 0,
};
export function backupFailure(e: unknown): BackupFailure {
  if (
    e &&
    typeof e === 'object' &&
    'code' in e &&
    'message' in e &&
    typeof e.code === 'string' &&
    typeof e.message === 'string' &&
    /^[a-z_]{1,50}$/.test(e.code)
  ) {
    return {
      code: e.code,
      message: e.message,
      diagnostic:
        'diagnostic' in e &&
        typeof e.diagnostic === 'string' &&
        /^WPD HRESULT 0x[0-9A-F]{8}$/.test(e.diagnostic)
          ? e.diagnostic
          : null,
    };
  }
  return {
    code: 'backup_unavailable',
    message:
      'Backup integration could not be reached. Open the Windows desktop app, check storage, and retry.',
    diagnostic: null,
  };
}
async function call<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (!isTauri())
    throw {
      code: 'runtime_unavailable',
      message:
        'USB backups require ChatVault Desktop on Windows. Browser preview does not copy phone files.',
      diagnostic: null,
    };
  try {
    return await invoke<T>(command, args);
  } catch (e) {
    throw backupFailure(e);
  }
}
export const backupService: BackupService = {
  available: isTauri,
  prepare: (deviceId, source, options) =>
    call('prepare_backup', { deviceId, source, options: options ?? null }),
  planFiles: (planId, category, offset) =>
    call('get_backup_plan_files', { planId, category, offset }),
  start: (planId, selection) => call('start_backup', { planId, selection }),
  progress: () => call('get_backup_progress'),
  cancel: () => call('cancel_backup'),
  history: (offset = 0) => call('list_backup_snapshots', { offset }),
  files: (snapshotId, offset) =>
    call('get_backup_files', { snapshotId, offset }),
  verify: (snapshotId) => call('reverify_backup', { snapshotId }),
  cleanup: (snapshotId) => call('cleanup_backup_staging', { snapshotId }),
  open: (snapshotId) => call('open_backup_location', { snapshotId }),
};
export function activePhase(phase: string) {
  return [
    'preparing',
    'scanning',
    'scan_waiting',
    'copying',
    'verifying',
    'committing',
    'verifying_local',
  ].includes(phase);
}
export function sourceLabel(source: Source | null) {
  return source === 'business' ? 'WhatsApp Business' : 'WhatsApp';
}
export function phaseLabel(phase: string) {
  return phase.replaceAll('_', ' ').replace(/^./, (s) => s.toUpperCase());
}
