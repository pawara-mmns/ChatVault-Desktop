// Synthetic interoperability vectors. Node/OpenSSL creates the ciphertext;
// production decryption uses RustCrypto, not this generator.
import { DatabaseSync } from 'node:sqlite';
import { createHash, createCipheriv, hkdfSync } from 'node:crypto';
import { deflateSync } from 'node:zlib';
import {
  mkdtempSync,
  mkdirSync,
  readFileSync,
  writeFileSync,
  rmSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
const out = new URL(
  '../src-tauri/src/storage/native/fixtures/',
  import.meta.url,
);
mkdirSync(out, { recursive: true });
const temp = mkdtempSync(join(tmpdir(), 'chatvault-synthetic-native-'));
const rootKey = Buffer.alloc(32, 7);
const salt = Buffer.alloc(32, 0x31);
const account = Buffer.alloc(16, 0x32);
const iv = Buffer.alloc(16, 0x33);
const png = Buffer.from(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVQIHWP4z8DwHwAFgAI/ScLbtAAAAABJRU5ErkJggg==',
  'base64',
);
const varint = (value) => {
  const b = [];
  do {
    let x = value & 127;
    value >>>= 7;
    if (value) x |= 128;
    b.push(x);
  } while (value);
  return Buffer.from(b);
};
const field = (id, value) =>
  typeof value === 'number'
    ? Buffer.concat([varint(id << 3), varint(value)])
    : Buffer.concat([varint((id << 3) | 2), varint(value.length), value]);
try {
  for (const format of ['crypt14', 'crypt15']) {
    const file = join(temp, format + '.sqlite');
    const db = new DatabaseSync(file);
    if (format === 'crypt14') {
      db.exec(
        "CREATE TABLE messages(_id INTEGER PRIMARY KEY,key_remote_jid TEXT,key_from_me INTEGER,key_id TEXT,data TEXT,timestamp INTEGER,remote_resource TEXT,media_wa_type INTEGER); INSERT INTO messages VALUES(1,'123@s.whatsapp.net',0,'legacy-1','Synthetic legacy incoming',1700000000000,NULL,0),(2,'123@s.whatsapp.net',1,'legacy-2','Synthetic legacy outgoing',1700000001000,NULL,0)",
      );
    } else {
      db.exec(
        "CREATE TABLE jid(_id INTEGER PRIMARY KEY,raw_string TEXT);CREATE TABLE chat(_id INTEGER PRIMARY KEY,jid_row_id INTEGER,subject TEXT);CREATE TABLE message(_id INTEGER PRIMARY KEY,chat_row_id INTEGER,sender_jid_row_id INTEGER,from_me INTEGER,timestamp INTEGER,message_type INTEGER,text_data TEXT,key_id TEXT);CREATE TABLE message_media(message_row_id INTEGER,file_path TEXT,file_size INTEGER,file_hash BLOB,mime_type TEXT);CREATE TABLE message_quoted(message_row_id INTEGER,key_id TEXT);INSERT INTO jid VALUES(1,'123@s.whatsapp.net'),(2,'group@g.us'),(3,'45@lid'),(4,'123@lid');INSERT INTO chat VALUES(1,1,NULL),(2,2,'Synthetic group'),(3,4,NULL)",
      );
      const add = db.prepare('INSERT INTO message VALUES(?,?,?,?,?,?,?,?)');
      add.run(1, 1, 1, 0, 1700000001000, 0, 'Synthetic incoming', 'modern-1');
      add.run(
        2,
        1,
        null,
        1,
        1700000002000,
        0,
        'සිංහල 😀 synthetic outgoing',
        'modern-2',
      );
      add.run(
        3,
        2,
        3,
        0,
        1700000000000,
        7,
        'Stored synthetic event',
        'modern-3',
      );
      add.run(4, 1, 1, 0, 1700000003000, 15, null, 'modern-4');
      add.run(
        5,
        1,
        1,
        0,
        1700000004000,
        1,
        'Synthetic photo caption',
        'modern-5',
      );
      add.run(6, 1, 1, 0, 1700000005000, 9, 'Unresolved document', 'modern-6');
      add.run(
        7,
        3,
        4,
        0,
        1700000006000,
        99,
        'Uninterpreted synthetic payload',
        'modern-7',
      );
      db.prepare('INSERT INTO message_media VALUES(?,?,?,?,?)').run(
        5,
        'Media/WhatsApp Business Images/synthetic.png',
        png.length,
        createHash('sha256').update(png).digest(),
        'image/png',
      );
      db.prepare('INSERT INTO message_media VALUES(?,?,?,?,?)').run(
        6,
        'Media/WhatsApp Business Documents/synthetic.png',
        png.length,
        createHash('sha256').update(png).digest(),
        'application/pdf',
      );
      db.exec("INSERT INTO message_quoted VALUES(2,'modern-1')");
    }
    db.close();
    const plaintext = readFileSync(file);
    const cipherFields =
      format === 'crypt14'
        ? Buffer.concat([
            field(1, Buffer.from([0, 1])),
            field(2, Buffer.from('2')),
            field(3, salt),
            field(4, account),
            field(5, iv),
          ])
        : field(1, iv);
    const prefix = Buffer.concat([
      field(1, format === 'crypt14' ? 0 : 1),
      field(format === 'crypt14' ? 2 : 3, cipherFields),
      field(6, format === 'crypt14' ? 0 : 3),
    ]);
    const header = Buffer.concat([varint(prefix.length), prefix]);
    const key =
      format === 'crypt14'
        ? rootKey
        : Buffer.from(
            hkdfSync(
              'sha256',
              rootKey,
              Buffer.alloc(32),
              'backup encryption',
              32,
            ),
          );
    const cipher = createCipheriv('aes-256-gcm', key, iv);
    const encrypted = Buffer.concat([
      cipher.update(deflateSync(plaintext)),
      cipher.final(),
    ]);
    const payload = Buffer.concat([header, encrypted, cipher.getAuthTag()]);
    writeFileSync(
      new URL(format === 'crypt14' ? 'legacy.crypt14' : 'modern.crypt15', out),
      Buffer.concat([payload, createHash('md5').update(payload).digest()]),
    );
  }
  writeFileSync(
    new URL('synthetic-key14.bin', out),
    Buffer.concat([
      Buffer.from([0, 1, 2]),
      salt,
      account,
      createHash('sha256').update(account).digest(),
      Buffer.alloc(16),
      rootKey,
    ]),
  );
  writeFileSync(new URL('synthetic.png', out), png);
} finally {
  // Only this directly-created temporary directory is removed.
  rmSync(temp, { recursive: true });
}
console.log(
  'Generated synthetic native vectors; no private keys or backups were used.',
);
