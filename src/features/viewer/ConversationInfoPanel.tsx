import { useEffect, useState } from 'react';
import { Image as ImageIcon, X, FileText } from 'lucide-react';
import {
  viewerService,
  viewerFailure,
  type Conversation,
  type MediaPage,
} from '../../services/viewer';
import { formatBytes } from '../../services/storage';
import { Button } from '../../shared/components/ui/button';
export function ConversationInfoPanel({
  conversation,
  onClose,
  onGallery,
}: {
  conversation: Conversation;
  onClose: () => void;
  onGallery: (page: MediaPage, index: number) => void;
}) {
  const [page, setPage] = useState<MediaPage | null>(null);
  const [offset, setOffset] = useState(0);
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(true);
  const [retry, setRetry] = useState(0);
  useEffect(() => {
    let active = true;
    viewerService
      .media(conversation.id, offset)
      .then((result) => {
        if (active) setPage(result);
      })
      .catch((e) => {
        if (active) setError(viewerFailure(e).message);
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [conversation.id, offset, retry]);
  return (
    <aside className="conversation-info" aria-label="Conversation information">
      <div className="reader-section-title">
        <h2>Conversation details</h2>
        <Button
          variant="ghost"
          size="icon"
          aria-label="Close conversation information"
          onClick={onClose}
        >
          <X size={16} />
        </Button>
      </div>
      <div className="info-heading">
        <span className="conversation-initial" aria-hidden="true">
          {Array.from(conversation.title)[0]}
        </span>
        <h3>{conversation.title}</h3>
        <span>Imported chat archive</span>
      </div>
      <dl className="conversation-facts">
        <div>
          <dt>Records</dt>
          <dd>{conversation.messageCount.toLocaleString()}</dd>
        </div>
        <div>
          <dt>Attachment references</dt>
          <dd>{conversation.attachmentCount.toLocaleString()}</dd>
        </div>
        <div>
          <dt>Stored media</dt>
          <dd>{conversation.mediaCount.toLocaleString()}</dd>
        </div>
        <div>
          <dt>Imported</dt>
          <dd>{new Date(conversation.importedAt).toLocaleString()}</dd>
        </div>
        <div>
          <dt>Original export</dt>
          <dd>{conversation.originalFilename ?? 'Unavailable'}</dd>
        </div>
        <div>
          <dt>Date order</dt>
          <dd>{conversation.dateOrder ?? 'Unknown'} · timezone unknown</dd>
        </div>
      </dl>
      <section>
        <h3>
          Participants <span>{conversation.participants.length}</span>
        </h3>
        {conversation.participants.length ? (
          <ul className="participant-list">
            {conversation.participants.map((name) => (
              <li key={name}>{name}</li>
            ))}
          </ul>
        ) : (
          <p className="reader-caption">
            No reliable participant names were recorded.
          </p>
        )}
      </section>
      <section>
        <h3>Local media</h3>
        <p className="reader-caption">
          Loaded only when you choose an attachment. Original filenames are
          retained.
        </p>
        {loading && <p role="status">Reading media metadata…</p>}
        {error && (
          <div role="alert">
            <p>{error}</p>
            <Button
              size="sm"
              variant="outline"
              onClick={() => {
                setLoading(true);
                setError('');
                setRetry((v) => v + 1);
              }}
            >
              Retry media list
            </Button>
          </div>
        )}
        {page?.items.length === 0 && !loading && (
          <p className="reader-caption">No extracted media in this archive.</p>
        )}
        <ul className="media-gallery">
          {page?.items.map((item, index) => (
            <li key={item.id}>
              <button onClick={() => onGallery(page, index)}>
                {item.mediaType?.startsWith('image/') ? (
                  <ImageIcon size={17} aria-hidden="true" />
                ) : (
                  <FileText size={17} aria-hidden="true" />
                )}
                <span>
                  <strong>{item.filename}</strong>
                  <small>
                    {item.sizeBytes === null
                      ? 'Size unavailable'
                      : formatBytes(item.sizeBytes)}
                  </small>
                </span>
              </button>
            </li>
          ))}
        </ul>
        {page && (offset > 0 || page.nextOffset !== null) && (
          <div className="reader-pagination">
            <Button
              size="sm"
              variant="outline"
              disabled={loading || offset === 0}
              onClick={() => {
                setLoading(true);
                setError('');
                setOffset((v) => Math.max(0, v - 50));
              }}
            >
              Previous media
            </Button>
            <Button
              size="sm"
              variant="outline"
              disabled={loading || page.nextOffset === null}
              onClick={() => {
                setLoading(true);
                setError('');
                setOffset(page.nextOffset!);
              }}
            >
              More media
            </Button>
          </div>
        )}
      </section>
      <p className="reader-caption">
        Stored locally without ChatVault encryption.
      </p>
    </aside>
  );
}
