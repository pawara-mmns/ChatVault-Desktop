import { useEffect, useRef, useState } from 'react';
import { Search, ChevronLeft, ChevronRight, X } from 'lucide-react';
import {
  viewerService,
  viewerFailure,
  messageTime,
  type SearchHit,
  type SearchPage,
} from '../../services/viewer';
import { Button } from '../../shared/components/ui/button';
import { Highlight } from './Highlight';
export function ChatSearch({
  conversationId,
  onJump,
  onClose,
}: {
  conversationId: string | null;
  onJump: (hit: SearchHit, query: string) => void;
  onClose: () => void;
}) {
  const [input, setInput] = useState('');
  const [scope, setScope] = useState<'all' | 'conversation'>(
    conversationId ? 'conversation' : 'all',
  );
  const [query, setQuery] = useState('');
  const [page, setPage] = useState<SearchPage | null>(null);
  const [cursor, setCursor] = useState<string | null>(null);
  const [history, setHistory] = useState<(string | null)[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [position, setPosition] = useState(-1);
  const serial = useRef(0);
  useEffect(
    () => () => {
      serial.current += 1;
    },
    [],
  );
  async function run(
    value: string,
    next: string | null = null,
    prior: (string | null)[] = [],
  ) {
    const request = ++serial.current;
    setBusy(true);
    setError('');
    setQuery(value);
    setPosition(-1);
    try {
      const result = await viewerService.search(
        value,
        scope === 'conversation' ? conversationId : null,
        next,
      );
      if (request === serial.current) {
        setPage(result);
        setCursor(next);
        setHistory(prior);
      }
    } catch (e) {
      if (request === serial.current) {
        setPage(null);
        setError(viewerFailure(e).message);
      }
    } finally {
      if (request === serial.current) setBusy(false);
    }
  }
  function jump(index: number) {
    if (!page?.items[index]) return;
    setPosition(index);
    onJump(page.items[index], query);
  }
  return (
    <section className="chat-search" aria-label="Message search">
      <div className="reader-section-title">
        <h2>
          <Search size={16} aria-hidden="true" /> Search saved messages
        </h2>
        <Button
          size="icon"
          variant="ghost"
          aria-label="Close message search"
          onClick={onClose}
        >
          <X size={16} />
        </Button>
      </div>
      <form
        className="message-search-form"
        onSubmit={(e) => {
          e.preventDefault();
          if (input.trim()) void run(input.trim());
        }}
      >
        <input
          type="search"
          aria-label="Search message text or sender"
          placeholder="Text or sender…"
          maxLength={200}
          value={input}
          onChange={(e) => setInput(e.target.value)}
        />
        <select
          aria-label="Search scope"
          value={scope}
          disabled={busy}
          onChange={(e) => {
            serial.current += 1;
            setScope(e.target.value as typeof scope);
            setPage(null);
            setQuery('');
          }}
        >
          <option value="all">All archives</option>
          <option value="conversation" disabled={!conversationId}>
            This conversation
          </option>
        </select>
        <Button size="sm" disabled={busy || !input.trim()} type="submit">
          Search
        </Button>
      </form>
      {busy && <p role="status">Searching local records…</p>}
      {error && <p role="alert">{error}</p>}
      {page && !busy && (
        <>
          <div className="search-result-tools">
            <span role="status">
              {page.items.length} results on this page
              {page.nextCursor ? ' · more records available' : ''}
            </span>
            <Button
              variant="ghost"
              size="icon"
              aria-label="Previous search result"
              disabled={position <= 0}
              onClick={() => jump(position - 1)}
            >
              <ChevronLeft size={16} />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              aria-label="Next search result"
              disabled={position + 1 >= page.items.length}
              onClick={() => jump(position + 1)}
            >
              <ChevronRight size={16} />
            </Button>
          </div>
          {page.items.length === 0 && (
            <p>
              {page.nextCursor
                ? 'No matches in this scanned section. Continue searching to examine the next records.'
                : 'No matching messages.'}
            </p>
          )}
          <ol className="search-results">
            {page.items.map((hit, index) => (
              <li key={hit.messageId}>
                <button
                  className={position === index ? 'is-selected' : ''}
                  onClick={() => jump(index)}
                >
                  <strong>
                    {hit.conversationTitle}{' '}
                    <span>· {hit.sender ?? 'Unknown sender'}</span>
                  </strong>
                  <span>
                    <Highlight text={hit.snippet} query={query} />
                  </span>
                  <small>{messageTime(hit)}</small>
                </button>
              </li>
            ))}
          </ol>
          <div className="reader-pagination">
            <Button
              size="sm"
              variant="outline"
              disabled={!history.length}
              onClick={() =>
                void run(
                  query,
                  history[history.length - 1],
                  history.slice(0, -1),
                )
              }
            >
              Previous results page
            </Button>
            <span>
              {page.mode === 'bounded-scan'
                ? `${page.scanned} records examined · short-query scan`
                : 'Local substring index'}
            </span>
            <Button
              size="sm"
              variant="outline"
              disabled={!page.nextCursor}
              onClick={() =>
                void run(query, page.nextCursor, [...history, cursor])
              }
            >
              {page.mode === 'bounded-scan'
                ? 'Continue search'
                : 'More results'}
            </Button>
          </div>
        </>
      )}
    </section>
  );
}
