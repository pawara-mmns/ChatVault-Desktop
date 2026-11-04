import { invoke, isTauri } from '@tauri-apps/api/core';
export type DeviceAccess =
  | 'unchecked'
  | 'available'
  | 'unavailable'
  | 'permission_required'
  | 'disconnected'
  | 'unsupported'
  | 'failed';
export interface DeviceFailure {
  code: string;
  message: string;
  diagnostic: string | null;
}
export interface Finding {
  application: string;
  location: string;
  category: string;
  status: string;
  encryptedBackupFound: boolean | null;
  diagnostic: DeviceFailure | null;
}
export interface Discovery {
  status: string;
  checkedAt: number;
  examined: number;
  findings: Finding[];
  diagnostic: DeviceFailure | null;
}
export interface ConnectedDevice {
  id: string;
  name: string | null;
  manufacturer: string | null;
  category: string | null;
  androidIdentity: 'unknown' | 'driver_reported_android';
  transport: 'usb' | 'unavailable';
  access: DeviceAccess;
  detectedAt: number;
  checkedAt: number | null;
  diagnostic: DeviceFailure | null;
  discovery: Discovery | null;
  savedNoteId: string | null;
}
export interface DeviceList {
  devices: ConnectedDevice[];
  scannedAt: number | null;
  stale: boolean;
  busy: boolean;
}
export interface KnownDevice {
  id: string;
  name: string;
  category: string | null;
  lastConnectedAt: number;
  preferred: boolean;
}
export interface DeviceService {
  available(): boolean;
  list(): Promise<DeviceList>;
  refresh(): Promise<DeviceList>;
  details(id: string): Promise<ConnectedDevice>;
  check(id: string): Promise<ConnectedDevice>;
  discover(id: string): Promise<ConnectedDevice>;
  cancel(): Promise<void>;
  known(): Promise<KnownDevice[]>;
  remember(id: string, preferred: boolean): Promise<KnownDevice>;
}
const codes = new Set([
  'access_denied',
  'disconnected',
  'busy',
  'cancelled',
  'timeout',
  'limit',
  'unsupported',
  'invalid_handle',
  'storage',
  'failed',
  'access_unavailable',
]);
export function deviceFailure(value: unknown): DeviceFailure {
  if (
    value &&
    typeof value === 'object' &&
    'code' in value &&
    'message' in value &&
    typeof value.code === 'string' &&
    codes.has(value.code) &&
    typeof value.message === 'string'
  ) {
    const diagnostic =
      'diagnostic' in value &&
      typeof value.diagnostic === 'string' &&
      /^WPD HRESULT 0x[0-9A-F]{8}$/.test(value.diagnostic)
        ? value.diagnostic
        : null;
    return { code: value.code, message: value.message, diagnostic };
  }
  return {
    code: 'failed',
    message:
      'Device integration could not be reached. Open the Windows desktop app and refresh.',
    diagnostic: null,
  };
}
async function call<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (!isTauri())
    throw {
      code: 'unsupported',
      message:
        'Windows USB discovery is unavailable in browser preview. Open ChatVault Desktop on Windows.',
      diagnostic: null,
    };
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw deviceFailure(error);
  }
}
export const deviceService: DeviceService = {
  available: isTauri,
  list: () => call('list_connected_devices'),
  refresh: () => call('refresh_connected_devices'),
  details: (deviceId) => call('get_device_details', { deviceId }),
  check: (deviceId) => call('check_device_file_access', { deviceId }),
  discover: (deviceId) => call('discover_whatsapp_storage', { deviceId }),
  cancel: () => call('cancel_device_operation'),
  known: () => call('get_known_devices'),
  remember: (deviceId, preferred) =>
    call('remember_device', { deviceId, preferred }),
};
export const accessLabel: Record<DeviceAccess, string> = {
  unchecked: 'Portable device detected · file access not checked',
  available: 'File access available at last check',
  unavailable: 'Connection detected — file access unavailable',
  permission_required: 'Permission or authorization required',
  disconnected: 'Device disconnected',
  unsupported: 'Discovery unsupported by current adapter',
  failed: 'Unsupported or failed connection',
};
export function deviceTime(value: number | null) {
  return value === null ? 'Not yet scanned' : new Date(value).toLocaleString();
}
