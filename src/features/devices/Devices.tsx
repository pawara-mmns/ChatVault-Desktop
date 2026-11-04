import { Link } from 'react-router-dom';
import { useEffect, useState } from 'react';
import { Cable, Info, RefreshCw, Smartphone } from 'lucide-react';
import { Badge, PageHeading } from '../../shared/components/Page';
import { Button } from '../../shared/components/ui/button';
import {
  accessLabel,
  deviceFailure,
  deviceTime,
  type DeviceFailure,
  type KnownDevice,
} from '../../services/devices';
import { useDevices } from './DeviceProvider';
export function Devices() {
  const { list, busy, error, available, refresh, inspect, cancel, service } =
    useDevices();
  const [selected, setSelected] = useState<string | null>(null);
  const [consent, setConsent] = useState(false);
  const [preferred, setPreferred] = useState(false);
  const [known, setKnown] = useState<KnownDevice[]>([]);
  const [noteError, setNoteError] = useState<DeviceFailure | null>(null);
  const [saving, setSaving] = useState(false);
  const device = list.devices.find((d) => d.id === selected);
  useEffect(() => {
    let active = true;
    if (available)
      service
        .known()
        .then((v) => {
          if (active) setKnown(v);
        })
        .catch((e) => {
          if (active) setNoteError(deviceFailure(e));
        });
    return () => {
      active = false;
    };
  }, [available, service]);
  async function remember() {
    if (!device) return;
    setSaving(true);
    setNoteError(null);
    try {
      const note = await service.remember(device.id, preferred);
      setKnown((old) => [note, ...old.filter((n) => n.id !== note.id)]);
      setConsent(false);
    } catch (e) {
      setNoteError(deviceFailure(e));
    } finally {
      setSaving(false);
    }
  }
  return (
    <>
      <PageHeading
        eyebrow="YOUR CONNECTIONS"
        title="Devices"
        description="Detect local USB portable devices and inspect accessible WhatsApp folders."
        action={
          <Button
            disabled={!available || !!busy}
            onClick={() => void refresh()}
          >
            <RefreshCw size={16} aria-hidden="true" />
            Scan Devices / Refresh
          </Button>
        }
      />
      <section
        className="panel instructions"
        aria-labelledby="connection-guide"
      >
        <div className="panel-heading">
          <h2 id="connection-guide">Connect your Android phone</h2>
          <Cable size={19} aria-hidden="true" />
        </div>
        <ol className="instruction-list">
          <li>
            <span>1</span>
            <div>
              <h3>Connect and unlock</h3>
              <p>
                Use a USB cable that supports data transfer. Unlock your own
                phone.
              </p>
            </div>
          </li>
          <li>
            <span>2</span>
            <div>
              <h3>Allow File Transfer</h3>
              <p>
                Open the USB notification, choose File Transfer / MTP, and
                approve the phone’s file-access prompt when offered.
              </p>
            </div>
          </li>
          <li>
            <span>3</span>
            <div>
              <h3>Refresh, select, then discover</h3>
              <p>
                Click Scan Devices, select your device, then explicitly scan
                WhatsApp storage. USB debugging and administrator access are
                unnecessary.
              </p>
            </div>
          </li>
        </ol>
      </section>
      {!available && (
        <div className="notice">
          <Info aria-hidden="true" />
          <p>
            Windows USB discovery is unavailable in browser preview. Open
            ChatVault Desktop on Windows. No simulated devices are shown.
          </p>
        </div>
      )}
      <div role="status" className="device-scan-status">
        {busy ??
          (list.scannedAt === null
            ? 'Not yet scanned'
            : `Last scanned: ${deviceTime(list.scannedAt)}`)}{' '}
        {busy && (
          <Button size="sm" variant="outline" onClick={() => void cancel()}>
            Cancel scan
          </Button>
        )}
      </div>
      {error && <Failure failure={error} />}
      <div className="devices-grid">
        <section
          className="panel device-list"
          aria-labelledby="connected-heading"
          aria-busy={!!busy}
        >
          <div className="panel-heading">
            <h2 id="connected-heading">Connected devices</h2>
            <Badge>{list.devices.length} detected</Badge>
          </div>
          {list.devices.length === 0 ? (
            <div className="device-empty">
              <Smartphone size={36} aria-hidden="true" />
              <h3>
                {busy
                  ? 'Scanning…'
                  : error
                    ? 'Live device state unavailable'
                    : list.stale
                      ? 'Live device state not checked'
                      : 'No device detected'}
              </h3>
              <p>
                A charging-only connection may not appear in Windows Portable
                Devices. Unlock the phone, select File Transfer, and refresh.
              </p>
              {selected && !busy && (
                <p role="status">
                  The selected device disconnected or its live handle expired.
                </p>
              )}
            </div>
          ) : (
            <ul className="connected-device-list">
              {list.devices.map((d) => (
                <li key={d.id}>
                  <button
                    type="button"
                    aria-pressed={selected === d.id}
                    disabled={!!busy}
                    onClick={() => {
                      setSelected(d.id);
                      setConsent(false);
                      setPreferred(false);
                    }}
                  >
                    <strong>{d.name ?? 'Name unavailable'}</strong>
                    <span>{accessLabel[d.access]}</span>
                    <small>
                      {d.androidIdentity === 'unknown'
                        ? 'Device identity unknown'
                        : 'Android reported by driver'}{' '}
                      · {d.manufacturer ?? 'Manufacturer unavailable'}
                    </small>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </section>
        <section
          className="panel device-details"
          aria-labelledby="details-heading"
        >
          <h2 id="details-heading">Device details</h2>
          {device ? (
            <>
              <dl className="device-metadata">
                <dt>Friendly name</dt>
                <dd>{device.name ?? 'Unavailable'}</dd>
                <dt>Manufacturer</dt>
                <dd>{device.manufacturer ?? 'Unavailable'}</dd>
                <dt>Category</dt>
                <dd>{device.category ?? 'Unavailable'}</dd>
                <dt>Android identity</dt>
                <dd>
                  {device.androidIdentity === 'unknown'
                    ? 'Unknown — WPD does not establish the operating system'
                    : 'Explicitly reported by WPD driver'}
                </dd>
                <dt>Connection method</dt>
                <dd>
                  {device.transport === 'usb'
                    ? 'USB · Windows Portable Devices'
                    : 'Unavailable — local USB transport unconfirmed'}
                </dd>
                <dt>File access</dt>
                <dd>{accessLabel[device.access]}</dd>
                <dt>Last detected</dt>
                <dd>{deviceTime(device.detectedAt)}</dd>
                <dt>Last access check</dt>
                <dd>{deviceTime(device.checkedAt)}</dd>
                <dt>WhatsApp discovery</dt>
                <dd>{device.discovery?.status ?? 'Not requested'}</dd>
              </dl>
              {device.diagnostic && <Failure failure={device.diagnostic} />}
              <div className="device-actions">
                <Button
                  variant="outline"
                  disabled={!!busy || device.transport !== 'usb'}
                  onClick={() => void inspect(device.id, false)}
                >
                  Check file access
                </Button>
                <Button
                  disabled={!!busy || device.transport !== 'usb'}
                  onClick={() => void inspect(device.id, true)}
                >
                  Scan WhatsApp Storage
                </Button>
              </div>
              <p className="muted">
                Scan WhatsApp Storage authorizes a bounded folder and filename
                metadata check. File contents are never read. Folder access is
                checked at that moment and may change when the phone locks.
              </p>
              <fieldset className="device-consent" disabled={!!busy || saving}>
                <legend>Optional local device note</legend>
                <label>
                  <input
                    type="checkbox"
                    checked={consent}
                    onChange={(e) => setConsent(e.target.checked)}
                  />{' '}
                  I agree to save this display name, category, last detected
                  time, and preference locally.
                </label>
                <label>
                  <input
                    type="checkbox"
                    checked={preferred}
                    onChange={(e) => setPreferred(e.target.checked)}
                  />{' '}
                  Mark as preferred
                </label>
                <Button
                  size="sm"
                  variant="outline"
                  disabled={!consent}
                  onClick={() => void remember()}
                >
                  {saving ? 'Saving…' : 'Save device note'}
                </Button>
                <p>
                  No hardware identifier is saved. This note cannot
                  automatically recognize a phone after disconnection or
                  restart.
                </p>
              </fieldset>
            </>
          ) : (
            <p>
              Select a live device to view genuine available metadata. A
              portable device or a phone category alone does not confirm
              Android.
            </p>
          )}
        </section>
      </div>
      {device?.discovery && (
        <section
          className="panel discovery-results"
          aria-labelledby="discovery-heading"
        >
          <div className="panel-heading">
            <h2 id="discovery-heading">WhatsApp storage discovery</h2>
            <Badge>{device.discovery.status}</Badge>
          </div>
          <p>
            Checked {deviceTime(device.discovery.checkedAt)} ·{' '}
            {device.discovery.examined} object metadata entries examined.
            Findings describe this scan only.
          </p>
          {device.discovery.diagnostic && (
            <Failure failure={device.discovery.diagnostic} />
          )}
          <ul>
            {device.discovery.findings.map((f, i) => (
              <li key={i}>
                <strong>
                  {f.application} · {f.category.replaceAll('_', ' ')}
                </strong>
                <span>{findingStatus(f.status)}</span>
                <p>{f.location}</p>
                {f.encryptedBackupFound === true && (
                  <p>
                    Encrypted local backup filename found. Contents were not
                    opened; this is not a readable chat archive.
                  </p>
                )}
                {f.diagnostic && <Failure failure={f.diagnostic} />}
              </li>
            ))}
          </ul>
          <p>
            “Not found” applies only to the accessible candidate path. Access
            restrictions, vendor layouts, and scan limits cannot establish that
            WhatsApp data is absent.
          </p>
        </section>
      )}
      {noteError && <Failure failure={noteError} />}
      {known.length > 0 && (
        <section className="panel device-notes">
          <h2>Saved device notes</h2>
          <p>
            Historical notes only. These records do not imply a current
            connection or stable phone identity.
          </p>
          <ul>
            {known.map((n) => (
              <li key={n.id}>
                <strong>{n.name}</strong> ·{' '}
                {n.category ?? 'Category unavailable'}
                {n.preferred ? ' · Preferred' : ''} · Last detected{' '}
                {deviceTime(n.lastConnectedAt)}
              </li>
            ))}
          </ul>
        </section>
      )}
      <div className="notice">
        <Info size={20} aria-hidden="true" />
        <div>
          <strong>Read-only discovery · Step 05</strong>
          <p>
            Current and legacy WhatsApp / WhatsApp Business paths are
            candidates, not guarantees. Private app storage is inaccessible. No
            backup is created, and no encrypted database or media bytes are
            read. Discovery is limited to 2,500 entries and 25 seconds between
            Windows calls; a blocked vendor call may take longer to return. ADB
            is not included or installed.
          </p>
          <Link to="/backups" className="text-link">
            Backup Now · Choose files and review
          </Link>
        </div>
      </div>
    </>
  );
}
function Failure({ failure }: { failure: DeviceFailure }) {
  return (
    <div role="alert" className="device-error">
      <strong>{failure.message}</strong>
      <small>
        Diagnostic category: {failure.code}
        {failure.diagnostic ? ` · ${failure.diagnostic}` : ''}
      </small>
    </div>
  );
}
function findingStatus(status: string) {
  return (
    (
      {
        found: 'Candidate folder found',
        accessible: 'Folder accessible',
        not_found: 'Folder not found in checked location',
        access_denied: 'Access denied',
        disconnected: 'Device disconnected',
        limit: 'Scan incomplete — entry limit',
        timeout: 'Scan incomplete — time limit',
        unsupported: 'Discovery unsupported',
        cancelled: 'Scan cancelled',
      } as Record<string, string>
    )[status] ?? 'Scan incomplete'
  );
}
