import { Link } from 'react-router-dom';
import {
  Archive,
  ArrowRight,
  ArrowUpFromLine,
  Database,
  HardDrive,
  LockKeyhole,
  ShieldCheck,
  Smartphone,
} from 'lucide-react';
import { Badge, EmptyState, PageHeading } from '../../shared/components/Page';
import { buttonVariants } from '../../shared/components/ui/button';
export function Dashboard() {
  return (
    <>
      <PageHeading
        eyebrow="YOUR PERSONAL ARCHIVE"
        title="A home for your conversations."
        description="Keep your chat history close. Keep control of your data."
        action={
          <Link to="/import" className={buttonVariants()}>
            <ArrowUpFromLine size={16} aria-hidden="true" />
            Import Chat
          </Link>
        }
      />
      <section className="welcome-panel" aria-labelledby="welcome-heading">
        <div className="welcome-copy">
          <Badge accent>PRIVATE BY DESIGN</Badge>
          <h2 id="welcome-heading">Your memories, on your terms.</h2>
          <p>
            A quiet, private space for the conversations that matter.
            <br className="desktop-break" /> Start by learning how to bring your
            chat exports here.
          </p>
          <Link to="/import" className="text-link">
            Explore the import workflow{' '}
            <ArrowRight size={15} aria-hidden="true" />
          </Link>
        </div>
        <div className="vault-art" aria-hidden="true">
          <div className="vault-outline">
            <ShieldCheck size={58} strokeWidth={1} />
          </div>
          <span>
            <LockKeyhole size={11} /> STORED WITH YOU
          </span>
        </div>
      </section>
      <section className="summary-grid" aria-label="Backup summary">
        <article className="summary-card">
          <div className="summary-title">
            Backup snapshots
            <HardDrive size={18} aria-hidden="true" />
          </div>
          <strong className="summary-value">0</strong>
          <p>No backups created</p>
        </article>
        <article className="summary-card">
          <div className="summary-title">
            Chat archives
            <Archive size={18} aria-hidden="true" />
          </div>
          <strong className="summary-value">0</strong>
          <p>No chats imported</p>
        </article>
        <article className="summary-card">
          <div className="summary-title">
            Storage used
            <Database size={18} aria-hidden="true" />
          </div>
          <strong className="summary-value" aria-label="Not calculated">
            —
          </strong>
          <p>Available after your first backup</p>
        </article>
      </section>
      <div className="dashboard-grid">
        <section
          className="panel recent-panel"
          aria-labelledby="recent-heading"
        >
          <div className="panel-heading">
            <h2 id="recent-heading">Recent backups</h2>
            <Badge>0 snapshots</Badge>
          </div>
          <EmptyState
            icon={HardDrive}
            title="Your story starts here"
            description="Once backups are available, your snapshots will appear here. Nothing has been backed up yet."
          >
            <Link
              to="/import"
              className={buttonVariants({ variant: 'outline', size: 'sm' })}
            >
              See how it works
              <ArrowRight size={14} aria-hidden="true" />
            </Link>
          </EmptyState>
        </section>
        <section
          className="panel device-panel"
          aria-labelledby="device-heading"
        >
          <div className="panel-heading">
            <h2 id="device-heading">Your device</h2>
            <span className="status-dot" />
          </div>
          <div className="device-illustration">
            <Smartphone size={46} strokeWidth={1.2} aria-hidden="true" />
            <span className="device-cross">×</span>
          </div>
          <h3>Not connected</h3>
          <p>Connect your Android phone to get ready for future backups.</p>
          <Link to="/devices" className="text-link">
            Connection guide <ArrowRight size={14} aria-hidden="true" />
          </Link>
          <div className="device-footnote">
            Device detection is not yet available
          </div>
        </section>
      </div>
      <div className="privacy-line">
        <ShieldCheck size={17} aria-hidden="true" />
        <p>
          <strong>A little peace of mind.</strong> Your chats will never need to
          leave your computer. No uploads, no tracking, no account required.
        </p>
      </div>
    </>
  );
}
