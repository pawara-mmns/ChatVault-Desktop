import {
  FileText,
  FolderArchive,
  LockKeyhole,
  ArrowRight,
  Info,
} from 'lucide-react';
import { Link } from 'react-router-dom';
import { Badge, PageHeading } from '../../shared/components/Page';
import { Button } from '../../shared/components/ui/button';
import { availability } from '../../services/availability';
export function ImportChat() {
  return (
    <>
      <PageHeading
        eyebrow="BRING YOUR HISTORY"
        title="Import Chat"
        description="Understand your files before you bring them into your archive."
      />
      <div className="notice">
        <Info size={20} aria-hidden="true" />
        <div>
          <strong>A preview of the workflow</strong>
          <p>
            Chat import is not yet available. No files can be selected, read, or
            imported in this version.
          </p>
        </div>
        <Badge>Coming later</Badge>
      </div>
      <div className="import-grid">
        <section className="panel information-card">
          <div className="card-icon">
            <FileText size={23} aria-hidden="true" />
          </div>
          <h2>Chat exports</h2>
          <Badge accent>PLANNED IMPORT FORMAT</Badge>
          <p>
            A chat export is a readable text file created with WhatsApp’s Export
            chat option. Depending on how it is shared, media may be supplied
            separately or inside an archive.
          </p>
          <p>
            Future import support will parse supported text formats and
            associate media you explicitly select. Export limits and date
            formats can vary.
          </p>
          <Button
            disabled={!availability.chatImport}
            aria-describedby="import-unavailable"
          >
            <ArrowRight size={16} aria-hidden="true" />
            Choose chat export
          </Button>
          <p id="import-unavailable" className="helper-text">
            File selection and parsing are coming in a later step.
          </p>
        </section>
        <section className="panel information-card">
          <div className="card-icon neutral">
            <LockKeyhole size={23} aria-hidden="true" />
          </div>
          <h2>Encrypted native backups</h2>
          <Badge>NOT SUPPORTED</Badge>
          <p>
            Native WhatsApp database backups, including .crypt files, are
            encrypted. They are different from readable chat exports.
          </p>
          <p>
            ChatVault does not bypass encryption, recover keys, access private
            messaging APIs, or unlock protected app storage. Copying a backup
            file does not make its chats readable.
          </p>
          <div className="inline-note">
            <FolderArchive size={18} aria-hidden="true" />
            Accessible-file backup and readable chat import are separate
            workflows.
          </div>
        </section>
      </div>
      <section
        className="panel workflow-panel"
        aria-labelledby="workflow-heading"
      >
        <div className="panel-heading">
          <h2 id="workflow-heading">The planned workflow</h2>
          <Badge>Not yet available</Badge>
        </div>
        <ol className="workflow-steps">
          <li>
            <span>01</span>
            <h3>Export a conversation</h3>
            <p>Use the chat app’s export option on your own phone.</p>
          </li>
          <li>
            <span>02</span>
            <h3>Choose your files</h3>
            <p>Explicitly select a supported export and any included media.</p>
          </li>
          <li>
            <span>03</span>
            <h3>Browse locally</h3>
            <p>Read the imported conversation in your private archive.</p>
          </li>
        </ol>
      </section>
      <p className="section-caption">
        Looking for phone backups?{' '}
        <Link to="/devices" className="text-link">
          Read the device connection guide{' '}
          <ArrowRight size={14} aria-hidden="true" />
        </Link>
      </p>
    </>
  );
}
