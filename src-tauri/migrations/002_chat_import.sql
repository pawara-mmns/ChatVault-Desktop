ALTER TABLE archives ADD COLUMN original_filename TEXT;
ALTER TABLE archives ADD COLUMN source_format TEXT CHECK(source_format IN ('txt','zip'));
ALTER TABLE archives ADD COLUMN original_sha256 TEXT CHECK(original_sha256 IS NULL OR length(original_sha256)=64);
ALTER TABLE archives ADD COLUMN import_status TEXT NOT NULL DEFAULT 'legacy' CHECK(import_status IN ('legacy','complete'));
ALTER TABLE archives ADD COLUMN message_count INTEGER NOT NULL DEFAULT 0 CHECK(message_count>=0);
ALTER TABLE archives ADD COLUMN media_count INTEGER NOT NULL DEFAULT 0 CHECK(media_count>=0);
ALTER TABLE archives ADD COLUMN missing_media_count INTEGER NOT NULL DEFAULT 0 CHECK(missing_media_count>=0);
ALTER TABLE archives ADD COLUMN total_bytes INTEGER NOT NULL DEFAULT 0 CHECK(total_bytes>=0);
ALTER TABLE archives ADD COLUMN text_encoding TEXT;
ALTER TABLE archives ADD COLUMN date_order TEXT;
CREATE UNIQUE INDEX archives_original_sha256 ON archives(original_sha256) WHERE original_sha256 IS NOT NULL;

ALTER TABLE conversations ADD COLUMN participants_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE messages ADD COLUMN original_timestamp TEXT;
ALTER TABLE messages ADD COLUMN local_timestamp TEXT;
ALTER TABLE messages ADD COLUMN timezone_status TEXT NOT NULL DEFAULT 'unknown' CHECK(timezone_status='unknown');
ALTER TABLE messages ADD COLUMN classification TEXT NOT NULL DEFAULT 'unknown' CHECK(classification IN ('message','system','unknown','deleted'));
ALTER TABLE messages ADD COLUMN original_record TEXT;
ALTER TABLE messages ADD COLUMN sequence_number INTEGER;
CREATE UNIQUE INDEX messages_sequence ON messages(conversation_id,sequence_number) WHERE sequence_number IS NOT NULL;

-- Rebuild only the attachment schema inside the migration transaction, preserving existing rows.
-- Archive-local media paths are now supported; missing references have no stored path.
CREATE TABLE archive_media (
 id TEXT PRIMARY KEY NOT NULL,
 archive_id TEXT NOT NULL REFERENCES archives(id) ON DELETE CASCADE,
 original_filename TEXT NOT NULL,
 storage_path TEXT NOT NULL UNIQUE CHECK(storage_path LIKE 'archives/%' AND instr(storage_path,'..')=0 AND instr(storage_path,'\')=0 AND instr(storage_path,':')=0),
 media_type TEXT NOT NULL,
 file_size INTEGER NOT NULL CHECK(file_size>=0)
) STRICT;
CREATE INDEX archive_media_archive ON archive_media(archive_id);
CREATE TABLE attachments_v2 (
 id TEXT PRIMARY KEY NOT NULL CHECK(length(id)>0),
 message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
 media_type TEXT,
 storage_path TEXT CHECK(storage_path IS NULL OR ((storage_path LIKE 'media/%' OR storage_path LIKE 'archives/%') AND instr(storage_path,'..')=0 AND instr(storage_path,'\')=0 AND instr(storage_path,':')=0)),
 file_size INTEGER CHECK(file_size IS NULL OR file_size>=0),
 original_filename TEXT,
 archive_media_id TEXT REFERENCES archive_media(id),
 is_missing INTEGER NOT NULL DEFAULT 0 CHECK(is_missing IN (0,1)),
 CHECK((is_missing=1 AND storage_path IS NULL) OR (is_missing=0 AND storage_path IS NOT NULL))
) STRICT;
INSERT INTO attachments_v2(id,message_id,media_type,storage_path,file_size) SELECT id,message_id,media_type,storage_path,file_size FROM attachments;
DROP TABLE attachments;
ALTER TABLE attachments_v2 RENAME TO attachments;
CREATE INDEX attachments_message ON attachments(message_id);

CREATE TABLE import_diagnostics (
 id INTEGER PRIMARY KEY,
 archive_id TEXT NOT NULL REFERENCES archives(id) ON DELETE CASCADE,
 code TEXT NOT NULL,
 severity TEXT NOT NULL CHECK(severity IN ('warning','info')),
 description TEXT NOT NULL,
 occurrence_count INTEGER NOT NULL CHECK(occurrence_count>0),
 line_number INTEGER
) STRICT;
CREATE INDEX import_diagnostics_archive ON import_diagnostics(archive_id);

-- Durable intent, committed before creating final files. Never visible as an archive.
CREATE TABLE import_jobs (
 id TEXT PRIMARY KEY NOT NULL,
 vault_path TEXT NOT NULL,
 storage_path TEXT NOT NULL UNIQUE,
 installation_id TEXT NOT NULL,
 created_at INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec')*1000 AS INTEGER))
) STRICT;
