// Auxiliary SQL verification using Node 24's SQLite. Not a replacement for cargo test.
// Entirely in-memory: never opens the real user's database or vault.
import { DatabaseSync } from 'node:sqlite';
import { readFileSync } from 'node:fs';
import assert from 'node:assert/strict';

const database = new DatabaseSync(':memory:');
database.function('chatvault_fold', { deterministic: true }, (value) =>
  value.normalize('NFC').toLowerCase().normalize('NFC'),
);
database.exec(
  'PRAGMA foreign_keys = ON; PRAGMA trusted_schema = OFF; BEGIN IMMEDIATE;',
);
database.exec(
  readFileSync(
    new URL('../src-tauri/migrations/001_initial.sql', import.meta.url),
    'utf8',
  ),
);
database.exec(
  readFileSync(
    new URL('../src-tauri/migrations/002_chat_import.sql', import.meta.url),
    'utf8',
  ),
);
database.exec(
  readFileSync(
    new URL('../src-tauri/migrations/003_chat_viewer.sql', import.meta.url),
    'utf8',
  ),
);
database.exec(
  readFileSync(
    new URL('../src-tauri/migrations/004_device_notes.sql', import.meta.url),
    'utf8',
  ),
);
database.exec('COMMIT;');
for (const table of [
  'app_settings',
  'devices',
  'backup_snapshots',
  'archives',
  'conversations',
  'messages',
  'attachments',
]) {
  assert.equal(
    database.prepare(`SELECT count(*) AS count FROM ${table}`).get().count,
    0,
  );
}
assert.throws(
  () =>
    database.exec(
      "INSERT INTO messages(id,conversation_id,message_text) VALUES ('m','missing','test fixture')",
    ),
  /FOREIGN KEY/,
);
assert.throws(
  () =>
    database.exec(
      "INSERT INTO archives(id,title,source_type,storage_path) VALUES ('a','Fixture','chat-export','../outside')",
    ),
  /CHECK constraint/,
);
assert.throws(
  () =>
    database.exec(
      "INSERT INTO backup_snapshots(id,status,storage_path,total_bytes) VALUES ('b','complete','backups/b',-1)",
    ),
  /CHECK constraint/,
);
database.exec(
  "INSERT INTO app_settings(key,value) VALUES ('fixture','local');",
);
const saved = database
  .prepare("SELECT updated_at FROM app_settings WHERE key='fixture'")
  .get();
assert.equal(typeof saved.updated_at, 'number');
assert.ok(Math.abs(Date.now() - saved.updated_at) < 5000);
database.exec('BEGIN IMMEDIATE; CREATE TABLE must_rollback(id INTEGER);');
try {
  database.exec('INSERT INTO missing_table VALUES (1);');
} catch {
  database.exec('ROLLBACK;');
}
assert.equal(
  database
    .prepare(
      "SELECT count(*) AS count FROM sqlite_schema WHERE name='must_rollback'",
    )
    .get().count,
  0,
);
database.close();
console.log(
  'PASS: initial SQL schema, empty tables, UTC timestamp defaults, constraints, foreign keys, transactional DDL rollback (in-memory SQLite). Rust migration/service tests remain separate.',
);
