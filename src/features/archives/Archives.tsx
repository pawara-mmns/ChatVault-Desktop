import { useState } from 'react';
import { Archive, ArrowUpFromLine, Search, RotateCw } from 'lucide-react';
import { Link } from 'react-router-dom';
import { Badge, EmptyState, PageHeading } from '../../shared/components/Page';
import { Button, buttonVariants } from '../../shared/components/ui/button';
import { useArchives } from './useArchives';
import {
  archiveService,
  importFailure,
  type ArchiveSummary,
} from '../../services/imports';
import { formatBytes, type StorageFailure } from '../../services/storage';

export function Archives() {
  const [query, setQuery] = useState('');
  const [offset, setOffset] = useState(0);
  const { page, loading, error, refresh } = useArchives(query, offset);
  const [summary, setSummary] = useState<ArchiveSummary | null>(null);
  const [summaryError, setSummaryError] = useState<StorageFailure | null>(null);
  const [reading, setReading] = useState(false);
  async function inspect(id: string) {
    if (reading) return;
    setReading(true);
    setSummaryError(null);
    try {
      setSummary(await archiveService.summary(id));
    } catch (failure) {
      setSummaryError(importFailure(failure));
    } finally {
      setReading(false);
    }
  }
  return (
    <>
      <PageHeading
        eyebrow="YOUR LIBRARY"
        title="Chat archives"
        description="Imported conversation records and media, stored on this computer."
        action={
          <Link to="/import" className={buttonVariants()}>
            <ArrowUpFromLine size={16} aria-hidden="true" />
            Import Chat
          </Link>
        }
      />
      <section
        className="panel"
        aria-labelledby="library-heading"
        aria-busy={loading}
      >
        <div className="library-toolbar">
          <h2 id="library-heading">
            All archives <Badge>{page?.total ?? '—'}</Badge>
          </h2>
          <div className="search-field">
            <Search size={16} aria-hidden="true" />
            <input
              type="search"
              aria-label="Search archives"
              placeholder="Filter title or filename"
              maxLength={200}
              value={query}
              onChange={(event) => {
                setQuery(event.target.value);
                setOffset(0);
              }}
            />
          </div>
          <Button
            variant="ghost"
            size="icon"
            aria-label="Refresh archives"
            disabled={loading}
            onClick={refresh}
          >
            <RotateCw size={16} aria-hidden="true" />
          </Button>
        </div>
        <p className="toolbar-note">
          Filter archive titles and original filenames, or open the local reader
          to browse and search saved messages.{' '}
          <Link to="/archives/read">Open chat reader</Link>
        </p>
        {loading && (
          <p className="library-status" role="status">
            Loading local archives…
          </p>
        )}
        {error && (
          <div className="library-status" role="alert">
            <p>{error.message}</p>
            <Button variant="outline" size="sm" onClick={refresh}>
              Retry library
            </Button>
          </div>
        )}
        {!loading && page && page.items.length === 0 && (
          <EmptyState
            icon={Archive}
            title={query ? 'No matching archives' : 'No archives yet'}
            description={
              query
                ? 'Try another title or original filename.'
                : 'Import a compatible WhatsApp text or ZIP export to create your first local archive.'
            }
          >
            <Link
              to="/import"
              className={buttonVariants({ variant: 'outline' })}
            >
              Import Chat
            </Link>
          </EmptyState>
        )}
        {page && page.items.length > 0 && (
          <>
            <div className="archive-table-wrap">
              <table className="archive-table">
                <caption className="sr-only">Imported chat archives</caption>
                <thead>
                  <tr>
                    <th scope="col">Archive</th>
                    <th scope="col">Imported</th>
                    <th scope="col">Records / media</th>
                    <th scope="col">Size</th>
                    <th scope="col">Status</th>
                    <th scope="col">Details</th>
                  </tr>
                </thead>
                <tbody>
                  {page.items.map((item) => (
                    <tr key={item.id}>
                      <td>
                        <strong>{item.title}</strong>
                        <span>
                          {item.originalFilename ??
                            'Original filename unavailable'}
                        </span>
                      </td>
                      <td>{new Date(item.importedAt).toLocaleString()}</td>
                      <td>
                        {item.messageCount} / {item.mediaCount}
                      </td>
                      <td>{formatBytes(item.totalBytes)}</td>
                      <td>
                        <Badge>
                          {item.importStatus === 'complete'
                            ? 'Imported'
                            : 'Legacy record'}
                        </Badge>
                      </td>
                      <td>
                        <Link
                          to={`/archives/read?archive=${encodeURIComponent(item.id)}`}
                          className={buttonVariants({
                            variant: 'outline',
                            size: 'sm',
                          })}
                          aria-label={`Read ${item.title}`}
                        >
                          Read chat
                        </Link>{' '}
                        <Button
                          variant="outline"
                          size="sm"
                          disabled={reading}
                          aria-label={`Summary for ${item.title}`}
                          onClick={() => void inspect(item.id)}
                        >
                          Summary
                        </Button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <div className="library-pagination">
              <Button
                size="sm"
                variant="outline"
                onClick={() => setOffset((value) => Math.max(0, value - 20))}
                disabled={loading || offset === 0}
              >
                Previous
              </Button>
              <span>
                {offset + 1}–{Math.min(offset + page.items.length, page.total)}{' '}
                of {page.total}
              </span>
              <Button
                size="sm"
                variant="outline"
                onClick={() => setOffset((value) => value + 20)}
                disabled={loading || offset + page.items.length >= page.total}
              >
                Next
              </Button>
            </div>
          </>
        )}
      </section>
      {summaryError && (
        <p role="alert" className="error-text">
          {summaryError.message}
        </p>
      )}
      {summary && (
        <section
          className="panel information-card archive-summary"
          aria-labelledby="archive-summary-heading"
        >
          <div className="settings-title">
            <h2 id="archive-summary-heading">
              Archive summary: {summary.title}
            </h2>
            <Button variant="ghost" size="sm" onClick={() => setSummary(null)}>
              Close summary
            </Button>
          </div>
          <dl className="storage-details">
            <div>
              <dt>Original file</dt>
              <dd>{summary.originalFilename ?? 'Unavailable'}</dd>
            </div>
            <div>
              <dt>Records / media</dt>
              <dd>
                {summary.messageCount} / {summary.mediaCount}
              </dd>
            </div>
            <div>
              <dt>Missing references</dt>
              <dd>{summary.missingMediaCount}</dd>
            </div>
            <div>
              <dt>Format / encoding</dt>
              <dd>
                {summary.sourceFormat ?? 'Legacy'} /{' '}
                {summary.textEncoding ?? 'Unknown'}
              </dd>
            </div>
            <div>
              <dt>Date order</dt>
              <dd>{summary.dateOrder ?? 'Unknown'} · timezone unknown</dd>
            </div>
            <div>
              <dt>Original SHA-256</dt>
              <dd className="storage-path">
                {summary.originalSha256 ?? 'Unavailable'}
              </dd>
            </div>
          </dl>
          {summary.diagnostics.length > 0 && (
            <ul>
              {summary.diagnostics.map((item) => (
                <li key={item.code}>
                  {item.description} ({item.occurrenceCount})
                </li>
              ))}
            </ul>
          )}
          <p className="helper-text">
            <Link
              to={`/archives/read?archive=${encodeURIComponent(summary.id)}`}
            >
              Open this archive in the chat reader
            </Link>
          </p>
        </section>
      )}
    </>
  );
}
