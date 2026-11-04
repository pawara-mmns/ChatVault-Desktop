import { useEffect, useRef, useState } from 'react';
import {
  X,
  ZoomIn,
  ZoomOut,
  Maximize,
  ChevronLeft,
  ChevronRight,
} from 'lucide-react';
import { privateMediaUrl, type Attachment } from '../../services/viewer';
import { formatBytes } from '../../services/storage';
import { Button } from '../../shared/components/ui/button';
import { usePrivateMedia } from './MediaMessage';
export function MediaLightbox({
  items,
  index,
  onChange,
  onClose,
}: {
  items: Attachment[];
  index: number;
  onChange: (index: number) => void;
  onClose: () => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const [zoom, setZoom] = useState(1);
  const [decodeError, setDecodeError] = useState(false);
  const item = items[index];
  const { access, error, busy } = usePrivateMedia(item?.mediaId ?? null, true);
  useEffect(() => {
    const element = dialog.current;
    const previous =
      document.activeElement instanceof HTMLElement
        ? document.activeElement
        : null;
    element?.showModal?.();
    return () => {
      element?.close?.();
      previous?.focus();
    };
  }, []);
  return (
    <dialog
      ref={dialog}
      className="media-lightbox"
      aria-modal="true"
      aria-labelledby="lightbox-title"
      open={
        typeof HTMLDialogElement === 'undefined' ||
        !HTMLDialogElement.prototype.showModal
          ? true
          : undefined
      }
      onCancel={(e) => {
        e.preventDefault();
        onClose();
      }}
      onKeyDown={(e) => {
        if (e.key === 'ArrowLeft' && index > 0) {
          onChange(index - 1);
          setZoom(1);
          setDecodeError(false);
        }
        if (e.key === 'ArrowRight' && index + 1 < items.length) {
          onChange(index + 1);
          setZoom(1);
          setDecodeError(false);
        }
      }}
    >
      <div className="lightbox-toolbar">
        <div>
          <h2 id="lightbox-title">{item?.filename ?? 'Image'}</h2>
          <p>
            {access
              ? `${formatBytes(access.metadata.sizeBytes)} · ${access.metadata.width ?? '?'} × ${access.metadata.height ?? '?'}`
              : 'Private local media'}
          </p>
        </div>
        <Button
          variant="ghost"
          size="icon"
          aria-label="Zoom out"
          disabled={zoom <= 0.5}
          onClick={() => setZoom((v) => Math.max(0.5, v - 0.25))}
        >
          <ZoomOut size={18} />
        </Button>
        <span>{Math.round(zoom * 100)}%</span>
        <Button
          variant="ghost"
          size="icon"
          aria-label="Zoom in"
          disabled={zoom >= 4}
          onClick={() => setZoom((v) => Math.min(4, v + 0.25))}
        >
          <ZoomIn size={18} />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          aria-label="Fit image to screen"
          onClick={() => setZoom(1)}
        >
          <Maximize size={18} />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          aria-label="Close image viewer"
          onClick={onClose}
        >
          <X size={20} />
        </Button>
      </div>
      <div className="lightbox-canvas">
        {busy && <p role="status">Loading local image…</p>}
        {(error || decodeError) && (
          <p role="alert">
            {error ||
              'This image could not be decoded. The original remains unchanged.'}
          </p>
        )}
        {access &&
          !error &&
          !decodeError &&
          access.metadata.kind === 'image' &&
          item && (
            <img
              src={privateMediaUrl(access.token)}
              alt={item.filename}
              style={{ transform: `scale(${zoom})` }}
              onError={() => setDecodeError(true)}
            />
          )}
      </div>
      <div className="lightbox-navigation">
        <Button
          variant="outline"
          aria-label="Previous image"
          disabled={index === 0}
          onClick={() => {
            setZoom(1);
            setDecodeError(false);
            onChange(index - 1);
          }}
        >
          <ChevronLeft size={17} />
          Previous
        </Button>
        <span>
          {index + 1} of {items.length} in this media page
        </span>
        <Button
          variant="outline"
          aria-label="Next image"
          disabled={index + 1 >= items.length}
          onClick={() => {
            setZoom(1);
            setDecodeError(false);
            onChange(index + 1);
          }}
        >
          Next
          <ChevronRight size={17} />
        </Button>
      </div>
    </dialog>
  );
}
