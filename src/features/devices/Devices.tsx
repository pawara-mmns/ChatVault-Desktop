import { Cable, Info, Smartphone } from 'lucide-react';
import { Badge, PageHeading } from '../../shared/components/Page';
export function Devices() {
  return (
    <>
      <PageHeading
        eyebrow="YOUR CONNECTIONS"
        title="Devices"
        description="Prepare your Android phone for future accessible-file backups."
      />
      <section className="panel connection-panel">
        <div className="connection-icon">
          <Smartphone size={45} strokeWidth={1.3} aria-hidden="true" />
        </div>
        <div>
          <Badge>NOT CONNECTED</Badge>
          <h2>No device connected</h2>
          <p>
            Automated device detection is coming in a later step. This version
            cannot detect a phone, even when one is plugged in.
          </p>
        </div>
      </section>
      <section
        className="panel instructions"
        aria-labelledby="connection-guide"
      >
        <div className="panel-heading">
          <h2 id="connection-guide">Get ready to connect</h2>
          <Cable size={19} aria-hidden="true" />
        </div>
        <ol className="instruction-list">
          <li>
            <span>1</span>
            <div>
              <h3>Use your own Android phone</h3>
              <p>
                Unlock your phone and connect it to this computer using a USB
                cable that supports data transfer.
              </p>
            </div>
          </li>
          <li>
            <span>2</span>
            <div>
              <h3>Allow file transfer</h3>
              <p>
                On your phone, open the USB notification and choose File
                transfer / Android Auto if offered. Confirm any file-access
                prompt you recognize.
              </p>
            </div>
          </li>
          <li>
            <span>3</span>
            <div>
              <h3>Check accessible files in Windows</h3>
              <p>
                Use File Explorer to see folders your phone makes available.
                Visibility depends on your Android version, phone manufacturer,
                and app permissions.
              </p>
            </div>
          </li>
        </ol>
      </section>
      <div className="notice">
        <Info size={20} aria-hidden="true" />
        <div>
          <strong>Only what your phone allows</strong>
          <p>
            Future adapters will work within Android’s access rules. Private app
            storage, encryption bypass, root access, and automatic full-history
            recovery are outside this project’s scope. No USB debugging is
            required for this step.
          </p>
        </div>
      </div>
    </>
  );
}
