-- Keep all original export/archive tables and foreign keys intact.
ALTER TABLE archives ADD COLUMN native_snapshot_id TEXT REFERENCES backup_snapshots(id);
ALTER TABLE archives ADD COLUMN native_file_id TEXT REFERENCES backup_files(id);
ALTER TABLE archives ADD COLUMN native_format TEXT;
ALTER TABLE archives ADD COLUMN native_source_app TEXT;
ALTER TABLE archives ADD COLUMN native_parser_version INTEGER;
ALTER TABLE archives ADD COLUMN native_schema TEXT;
ALTER TABLE archives ADD COLUMN native_diagnostics_json TEXT;
-- Native provenance distinguishes archives while preserving the legacy source_type constraint.
CREATE UNIQUE INDEX native_snapshot_parser ON archives(native_snapshot_id,native_parser_version)
 WHERE native_snapshot_id IS NOT NULL;
ALTER TABLE messages ADD COLUMN native_id TEXT;
ALTER TABLE messages ADD COLUMN native_direction TEXT CHECK(native_direction IN ('incoming','outgoing','unknown'));
ALTER TABLE messages ADD COLUMN native_timestamp_ms INTEGER;
ALTER TABLE messages ADD COLUMN native_type INTEGER;
ALTER TABLE messages ADD COLUMN native_reply_key TEXT;
CREATE TABLE native_media_links (
 media_id TEXT PRIMARY KEY NOT NULL REFERENCES archive_media(id) ON DELETE CASCADE,
 snapshot_id TEXT NOT NULL REFERENCES backup_snapshots(id),
 backup_file_id TEXT NOT NULL REFERENCES backup_files(id),
 UNIQUE(media_id,snapshot_id,backup_file_id)
) STRICT;
