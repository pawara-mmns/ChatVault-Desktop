import { useEffect, useRef, useState } from 'react';
import {
  FileText,
  Image as ImageIcon,
  Play,
  ExternalLink,
  RotateCw,
} from 'lucide-react';
import {
  viewerService,
  viewerFailure,
  privateMediaUrl,
  type Attachment,
  type MediaAccess,
} from '../../services/viewer';
import { formatBytes } from '../../services/storage';
import { Button } from '../../shared/components/ui/button';

export function usePrivateMedia(mediaId: string | null, enabled: boolean) {
  const [access, setAccess] = useState<MediaAccess | null>(null);
  const [error, setError] = useState<{ id: string; message: string } | null>(
    null,
  );
  const [revision, setRevision] = useState(0);
  useEffect(() => {
    if (!enabled || !mediaId) return;
    let active = true;
    let token: string | null = null;
    viewerService
      .authorize(mediaId)
      .then((result) => {
        token = result.token;
        if (active) setAccess(result);
        else void viewerService.revoke(result.token).catch(() => {});
      })
      .catch((e) => {
        if (active)
          setError({ id: mediaId, message: viewerFailure(e).message });
      });
    return () => {
      active = false;
      if (token) void viewerService.revoke(token).catch(() => {});
    };
  }, [mediaId, enabled, revision]);
  const currentAccess = access?.metadata.id === mediaId ? access : null;
  const currentError = error?.id === mediaId ? error.message : '';
  return {
    access: currentAccess,
    error: currentError,
    busy: enabled && !currentAccess && !currentError,
    retry: () => {
      setAccess(null);
      setError(null);
      setRevision((v) => v + 1);
    },
  };
}
export function AudioPlayer({
  url,
  filename,
  onError,
}: {
  url: string;
  filename: string;
  onError: () => void;
}) {
  const player = useRef<HTMLAudioElement>(null);
  return (
    <div className="audio-player">
      {/* eslint-disable-next-line jsx-a11y/media-has-caption -- Exports contain no caption/transcript file; do not fabricate one. */}
      <audio
        ref={player}
        controls
        preload="metadata"
        src={url}
        aria-label={`Play audio ${filename}`}
        onError={onError}
      />
      <label>
        Speed{' '}
        <select
          aria-label={`Playback speed for ${filename}`}
          defaultValue="1"
          onChange={(e) => {
            if (player.current)
              player.current.playbackRate = Number(e.target.value);
          }}
        >
          <option value="0.75">0.75×</option>
          <option value="1">1×</option>
          <option value="1.5">1.5×</option>
          <option value="2">2×</option>
        </select>
      </label>
      <span className="reader-caption">
        No transcript was included in this export.
      </span>
    </div>
  );
}
export function MediaMessage({
  attachment,
  onEnlarge,
}: {
  attachment: Attachment;
  onEnlarge?: (id: string) => void;
}) {
  const [enabled, setEnabled] = useState(false);
  const [decodeError, setDecodeError] = useState(false);
  const [documentBusy, setDocumentBusy] = useState(false);
  const [documentError, setDocumentError] = useState('');
  const { access, error, busy, retry } = usePrivateMedia(
    attachment.mediaId,
    enabled,
  );
  const hintedImage = attachment.mediaType?.startsWith('image/');
  const hintedPlayable =
    hintedImage ||
    attachment.mediaType?.startsWith('audio/') ||
    attachment.mediaType?.startsWith('video/');
  async function openDocument() {
    if (!attachment.mediaId) return;
    setDocumentBusy(true);
    setDocumentError('');
    try {
      await viewerService.openDocument(attachment.mediaId);
    } catch (e) {
      setDocumentError(viewerFailure(e).message);
    } finally {
      setDocumentBusy(false);
    }
  }
  return (
    <div className="media-message">
      <div className="attachment-label">
        <FileText size={15} aria-hidden="true" />
        <strong>{attachment.filename}</strong>
        <span>{attachment.mediaType ?? 'Type unknown'}</span>
        <span>
          {attachment.sizeBytes === null
            ? 'Size unknown'
            : formatBytes(attachment.sizeBytes)}
        </span>
      </div>
      {attachment.missing || !attachment.mediaId ? (
        <p className="attachment-missing">
          Attachment unavailable · omitted or missing in this export.
        </p>
      ) : (
        <>
          {!enabled && hintedPlayable && (
            <Button
              size="sm"
              variant="outline"
              onClick={() => setEnabled(true)}
            >
              {hintedImage ? (
                <ImageIcon size={15} aria-hidden="true" />
              ) : (
                <Play size={15} aria-hidden="true" />
              )}
              {hintedImage ? 'Load image preview' : 'Load recording'}
            </Button>
          )}
          {!hintedPlayable && (
            <>
              <p className="reader-caption">
                Available reference · checked again before opening. Opens in
                your registered application.
              </p>
              <Button
                size="sm"
                variant="outline"
                disabled={documentBusy}
                onClick={() => void openDocument()}
              >
                <ExternalLink size={14} aria-hidden="true" />
                {documentBusy ? 'Checking document…' : 'Open document'}
              </Button>
            </>
          )}
          {busy && <p role="status">Checking private attachment…</p>}
          {(error || decodeError) && (
            <div role="alert" className="attachment-missing">
              <p>
                {error ||
                  'This file could not be decoded by this WebView. Its format or codec may be unsupported, or the file may be damaged.'}
              </p>
              <Button
                size="sm"
                variant="ghost"
                onClick={() => {
                  setDecodeError(false);
                  retry();
                }}
              >
                <RotateCw size={14} aria-hidden="true" />
                Retry attachment
              </Button>
            </div>
          )}
          {documentError && <p role="alert">{documentError}</p>}
          {access && !decodeError && !error && (
            <>
              {access.metadata.kind === 'image' && (
                <button
                  className="image-preview"
                  aria-label={`Enlarge ${attachment.filename}`}
                  onClick={() => onEnlarge?.(attachment.mediaId!)}
                >
                  <img
                    src={privateMediaUrl(access.token)}
                    alt={attachment.filename}
                    loading="lazy"
                    width={access.metadata.width ?? undefined}
                    height={access.metadata.height ?? undefined}
                    onError={() => setDecodeError(true)}
                  />
                  <span>Enlarge image</span>
                </button>
              )}
              {access.metadata.kind === 'audio' && (
                <AudioPlayer
                  url={privateMediaUrl(access.token)}
                  filename={attachment.filename}
                  onError={() => setDecodeError(true)}
                />
              )}
              {access.metadata.kind === 'video' && (
                <video
                  controls
                  preload="metadata"
                  src={privateMediaUrl(access.token)}
                  aria-label={`Play video ${attachment.filename}`}
                  onError={() => setDecodeError(true)}
                >
                  <track kind="captions" />
                </video>
              )}
            </>
          )}
        </>
      )}
    </div>
  );
}
