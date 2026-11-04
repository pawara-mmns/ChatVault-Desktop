import { useEffect, useState } from 'react';
import { Link, useSearchParams } from 'react-router-dom';
import { ArrowLeft, Search, PanelRight, MessageSquare, X } from 'lucide-react';
import {
  viewerService,
  viewerFailure,
  type Conversation,
  type SearchHit,
  type Attachment,
  type MediaPage,
} from '../../services/viewer';
import { Button, buttonVariants } from '../../shared/components/ui/button';
import { ConversationSidebar } from './ConversationSidebar';
import { MessageTimeline } from './MessageTimeline';
import { ConversationInfoPanel } from './ConversationInfoPanel';
import { ChatSearch } from './ChatSearch';
import { MediaLightbox } from './MediaLightbox';
import { MediaMessage } from './MediaMessage';
const selectionKey = 'chatvault:selected-conversation';
function savedSelection() {
  try {
    return sessionStorage.getItem(selectionKey);
  } catch {
    return null;
  }
}
export function ChatViewerPage() {
  const [params, setParams] = useSearchParams();
  const archive = params.get('archive');
  const selected =
    params.get('conversation') ?? (archive ? null : savedSelection());
  const target = params.get('message');
  const [searchOpen, setSearchOpen] = useState(false);
  const [query, setQuery] = useState('');
  const [jumpVersion, setJumpVersion] = useState(0);
  const [archiveError, setArchiveError] = useState('');
  const [gallery, setGallery] = useState<{
    items: Attachment[];
    index: number;
  } | null>(null);
  const [attachment, setAttachment] = useState<Attachment | null>(null);
  useEffect(() => {
    if (selected)
      try {
        sessionStorage.setItem(selectionKey, selected);
      } catch {
        /* Session storage is optional. */
      }
  }, [selected]);
  function select(id: string, message?: string) {
    try {
      sessionStorage.setItem(selectionKey, id);
    } catch {
      /* The URL still preserves selection. */
    }
    setParams({ conversation: id, ...(message ? { message } : {}) });
    setGallery(null);
    setAttachment(null);
  }
  useEffect(() => {
    if (!archive || params.get('conversation')) return;
    let active = true;
    viewerService
      .conversations('', archive, 0, 1)
      .then((page) => {
        if (!active) return;
        if (page.items[0])
          setParams({ conversation: page.items[0].id }, { replace: true });
        else setArchiveError('This archive has no saved conversation.');
      })
      .catch((e) => {
        if (active) setArchiveError(viewerFailure(e).message);
      });
    return () => {
      active = false;
    };
  }, [archive, params, setParams]);
  function jump(hit: SearchHit, value: string) {
    setJumpVersion((v) => v + 1);
    setQuery(value);
    select(hit.conversationId, hit.messageId);
  }
  async function enlarge(id: string) {
    try {
      const metadata = await viewerService.metadata(id);
      setGallery({
        items: [
          {
            id,
            mediaId: id,
            filename: metadata.filename,
            mediaType: metadata.mime,
            sizeBytes: metadata.sizeBytes,
            missing: false,
          },
        ],
        index: 0,
      });
    } catch (e) {
      setArchiveError(viewerFailure(e).message);
    }
  }
  function openGallery(page: MediaPage, index: number) {
    const item = page.items[index];
    if (!item) return;
    if (item.mediaType?.startsWith('image/')) {
      const images = page.items.filter((m) =>
        m.mediaType?.startsWith('image/'),
      );
      setGallery({
        items: images,
        index: images.findIndex((m) => m.id === item.id),
      });
    } else setAttachment(item);
  }
  return (
    <div className={`reader-page ${searchOpen ? 'search-is-open' : ''}`}>
      <div className="reader-page-heading">
        <div>
          <Link to="/archives" className="reader-back">
            <ArrowLeft size={14} aria-hidden="true" />
            Archive library
          </Link>
          <h1 tabIndex={-1}>Your conversations</h1>
          <p>Saved words. Kept close.</p>
        </div>
        <Button
          variant="outline"
          onClick={() => setSearchOpen((v) => !v)}
          aria-expanded={searchOpen}
        >
          <Search size={15} aria-hidden="true" />
          Search messages
        </Button>
      </div>
      {archiveError && (
        <p role="alert" className="reader-notice">
          {archiveError}
        </p>
      )}
      {searchOpen && (
        <ChatSearch
          conversationId={selected}
          onJump={jump}
          onClose={() => setSearchOpen(false)}
        />
      )}
      <div className="reader-grid">
        <ConversationSidebar
          selected={selected}
          onSelect={(id) => {
            setQuery('');
            setSearchOpen(false);
            select(id);
          }}
        />
        {selected ? (
          <ConversationReader
            key={selected}
            id={selected}
            target={target}
            query={query}
            jumpVersion={jumpVersion}
            onEnlarge={(id) => void enlarge(id)}
            onGallery={openGallery}
          />
        ) : (
          <section className="reader-empty">
            <MessageSquare size={38} aria-hidden="true" />
            <h2>Make room for a conversation.</h2>
            <p>
              Choose a saved chat from the sidebar, or import a readable export
              to begin.
            </p>
            <Link
              to="/import"
              className={buttonVariants({ variant: 'outline' })}
            >
              Import Chat
            </Link>
          </section>
        )}
      </div>
      {gallery && (
        <MediaLightbox
          items={gallery.items}
          index={gallery.index}
          onChange={(index) => setGallery({ ...gallery, index })}
          onClose={() => setGallery(null)}
        />
      )}
      {attachment && (
        <AttachmentDialog
          attachment={attachment}
          onClose={() => setAttachment(null)}
        />
      )}
    </div>
  );
}
function ConversationReader({
  id,
  target,
  query,
  onEnlarge,
  onGallery,
  jumpVersion,
}: {
  id: string;
  target: string | null;
  query: string;
  onEnlarge: (id: string) => void;
  onGallery: (page: MediaPage, index: number) => void;
  jumpVersion: number;
}) {
  const [conversation, setConversation] = useState<Conversation | null>(null);
  const [error, setError] = useState('');
  const [retry, setRetry] = useState(0);
  const [info, setInfo] = useState(false);
  useEffect(() => {
    let active = true;
    viewerService
      .conversation(id)
      .then((result) => {
        if (active) setConversation(result);
      })
      .catch((e) => {
        if (active) setError(viewerFailure(e).message);
      });
    return () => {
      active = false;
    };
  }, [id, retry]);
  return (
    <div className={`conversation-reader ${info ? 'with-info' : ''}`}>
      <section
        className="conversation-center"
        aria-label="Selected conversation"
      >
        {error ? (
          <div role="alert" className="reader-notice">
            <p>{error}</p>
            <Button
              variant="outline"
              onClick={() => {
                setError('');
                setConversation(null);
                setRetry((v) => v + 1);
              }}
            >
              Retry conversation
            </Button>
          </div>
        ) : !conversation ? (
          <p role="status" className="reader-notice">
            Opening saved conversation…
          </p>
        ) : (
          <>
            <header className="conversation-header">
              <span className="conversation-initial" aria-hidden="true">
                {Array.from(conversation.title)[0]}
              </span>
              <div>
                <h2>{conversation.title}</h2>
                <p>
                  {conversation.messageCount.toLocaleString()} records ·{' '}
                  {conversation.participants.length} known participants
                </p>
              </div>
              <Button
                variant="ghost"
                size="icon"
                aria-label={
                  info
                    ? 'Hide conversation information'
                    : 'Show conversation information'
                }
                aria-expanded={info}
                onClick={() => setInfo((v) => !v)}
              >
                <PanelRight size={18} />
              </Button>
            </header>
            <MessageTimeline
              key={`${target ?? 'latest'}:${jumpVersion}`}
              conversationId={id}
              targetId={target}
              query={query}
              onEnlarge={onEnlarge}
            />
          </>
        )}
      </section>
      {info && conversation && (
        <ConversationInfoPanel
          key={id}
          conversation={conversation}
          onClose={() => setInfo(false)}
          onGallery={onGallery}
        />
      )}
    </div>
  );
}
function AttachmentDialog({
  attachment,
  onClose,
}: {
  attachment: Attachment;
  onClose: () => void;
}) {
  // Native dialog focus containment and Escape handling match the image lightbox.
  const [element, setElement] = useState<HTMLDialogElement | null>(null);
  useEffect(() => {
    if (!element) return;
    const previous =
      document.activeElement instanceof HTMLElement
        ? document.activeElement
        : null;
    element.showModal?.();
    return () => {
      element.close?.();
      previous?.focus();
    };
  }, [element]);
  return (
    <dialog
      ref={setElement}
      className="attachment-dialog"
      aria-modal="true"
      aria-labelledby="attachment-dialog-title"
      open={!HTMLDialogElement.prototype.showModal ? true : undefined}
      onCancel={(e) => {
        e.preventDefault();
        onClose();
      }}
    >
      <div className="reader-section-title">
        <h2 id="attachment-dialog-title">Local attachment</h2>
        <Button
          variant="ghost"
          size="icon"
          aria-label="Close attachment"
          onClick={onClose}
        >
          <X size={18} />
        </Button>
      </div>
      <MediaMessage attachment={attachment} />
    </dialog>
  );
}
