# Security and privacy

## Step 01 data flow

The application reads no chats, media, backup files, device identifiers, or external accounts. It saves only an appearance value in the app-origin localStorage key `chatvault.appearance.v1`. Removing that key resets appearance; clearing WebView data may also reset it. There is no SQLite database in this version.

The Rust health command returns static application information and the OS name. It reads/writes no files, runs no subprocesses, and has no network operations. No user content, paths, secrets, or arbitrary diagnostic strings are logged by application code or displayed in backend errors.

## Permissions and native surface

- `build.rs` registers `health_check` with Tauri's generated command permissions.
- The capability grants only `allow-health-check`, scoped to the local `main` window. No remote URL capability is granted.
- No filesystem, shell, HTTP, SQL, opener, USB, or clipboard plugin is installed or enabled.
- The default opener plugin and broad starter permissions were removed.
- The app does not request administrator privileges. The NSIS configuration uses per-user installation.
- The production CSP permits bundled resources and Tauri IPC; it has no external network destination. Inline styling is allowed for Vite/theme styling; inline scripts, frames, object embeds, and form submissions are not enabled.
- Development CSP additionally permits the loopback Vite/HMR connection. Vite binds to `127.0.0.1`, not all network interfaces.

No runtime service, cloud database, remote fonts, external image assets, analytics, login, or update endpoint is configured. Package installation and native build tools can require network access during development; this is separate from packaged runtime behavior.

## Honest capability states

Import and search controls are disabled. The Devices page says detection is unavailable even if a phone is physically connected. Zero counts are an uninitialized library foundation; they are not a claim that a phone or disk has been scanned. Storage is unmeasured. Backend failure has a visible state and retry action. Browser previews never produce a fabricated Rust success response.

## Future implementation requirements

Keep all content access behind service/repository boundaries. Introduce file permissions only with explicit selections and narrow scopes; do not add unrestricted frontend path access. Validate and bound untrusted exports, attachment paths, archive extraction, MIME types, sizes, and SQL parameters. Protect against traversal, decompression bombs, malformed timestamps, and HTML/script content before enabling import or media display. Never log chat bodies, sensitive files, keys, or backup secrets.

Future local-only storage is not automatically encryption at rest. The current project makes no claim of encrypted archive storage. Database encryption, key handling, retention, recovery, and secure deletion require separate design and review.

## Data-access limitations

Android file transfer exposes only what the phone and OS permit. Scoped storage and app-private directories can prevent access. A copied encrypted native backup remains encrypted. ChatVault does not recover encryption keys, bypass device protections, use private messaging APIs, require root, scrape WhatsApp Web, or promise full-history recovery.

ChatVault Desktop is an independent, unofficial product and is not affiliated with WhatsApp or Meta.
