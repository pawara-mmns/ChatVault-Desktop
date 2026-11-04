import { vi } from 'vitest';
import type { ConnectedDevice, DeviceService } from '../services/devices';
/** Synthetic metadata is compiled only into test modules. */
export function phone(
  overrides: Partial<ConnectedDevice> = {},
): ConnectedDevice {
  return {
    id: 'fixture-handle-1',
    name: 'Synthetic phone',
    manufacturer: 'Synthetic maker',
    category: null,
    androidIdentity: 'unknown',
    transport: 'usb',
    access: 'unchecked',
    detectedAt: 1000,
    checkedAt: null,
    diagnostic: null,
    discovery: null,
    savedNoteId: null,
    ...overrides,
  };
}
export function deviceFixture(devices: ConnectedDevice[] = []): DeviceService {
  const list = { devices, scannedAt: 1000, stale: false, busy: false };
  return {
    available: () => true,
    list: vi.fn().mockResolvedValue(list),
    refresh: vi.fn().mockResolvedValue(list),
    details: vi.fn().mockResolvedValue(devices[0]),
    check: vi
      .fn()
      .mockResolvedValue(
        phone({ access: 'available', category: 'Phone', checkedAt: 1001 }),
      ),
    discover: vi.fn().mockResolvedValue(
      phone({
        access: 'available',
        discovery: {
          status: 'complete',
          checkedAt: 1002,
          examined: 12,
          diagnostic: null,
          findings: [
            {
              application: 'WhatsApp',
              category: 'images',
              location:
                'Storage 1 / Android / media / com.whatsapp / WhatsApp / Media / WhatsApp Images',
              status: 'accessible',
              encryptedBackupFound: null,
              diagnostic: null,
            },
          ],
        },
      }),
    ),
    cancel: vi.fn().mockResolvedValue(undefined),
    known: vi.fn().mockResolvedValue([]),
    remember: vi.fn().mockResolvedValue({
      id: 'note',
      name: 'Synthetic phone',
      category: null,
      lastConnectedAt: 1000,
      preferred: false,
    }),
  };
}
