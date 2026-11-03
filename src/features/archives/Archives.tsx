import { Archive, ArrowUpFromLine, Search } from 'lucide-react';
import { Link } from 'react-router-dom';
import { Badge, EmptyState, PageHeading } from '../../shared/components/Page';
import { buttonVariants } from '../../shared/components/ui/button';
import { availability } from '../../services/availability';
export function Archives() {
  return (
    <>
      <PageHeading
        eyebrow="YOUR LIBRARY"
        title="Chat archives"
        description="A readable home for your exported conversations and shared memories."
        action={
          <Link to="/import" className={buttonVariants()}>
            <ArrowUpFromLine size={16} aria-hidden="true" />
            Import Chat
          </Link>
        }
      />
      <section className="panel" aria-labelledby="library-heading">
        <div className="library-toolbar">
          <h2 id="library-heading">
            All archives <Badge>0</Badge>
          </h2>
          <div className="search-field">
            <Search size={16} aria-hidden="true" />
            <input
              type="search"
              aria-label="Search archives"
              placeholder="Search archives"
              disabled={!availability.archiveSearch}
              aria-describedby="search-note"
            />
          </div>
        </div>
        <p id="search-note" className="toolbar-note">
          Archive search will be available after chat imports are implemented.
        </p>
        <EmptyState
          icon={Archive}
          title="No archives yet"
          description="Your imported chat exports will appear here. Importing and browsing chats are coming in a later step."
        >
          <Link to="/import" className={buttonVariants({ variant: 'outline' })}>
            Explore Import Chat
          </Link>
        </EmptyState>
      </section>
      <p className="section-caption">
        No chat files have been read or indexed. Archive storage is not yet
        implemented.
      </p>
    </>
  );
}
