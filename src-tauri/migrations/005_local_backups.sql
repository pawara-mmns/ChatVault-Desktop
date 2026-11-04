-- Preserve the original table and its foreign keys; extend its durable summary.
ALTER TABLE backup_snapshots ADD COLUMN source_app TEXT CHECK(source_app IN ('whatsapp','business'));
ALTER TABLE backup_snapshots ADD COLUMN device_name TEXT;
ALTER TABLE backup_snapshots ADD COLUMN phase TEXT NOT NULL DEFAULT 'interrupted';
ALTER TABLE backup_snapshots ADD COLUMN file_count INTEGER NOT NULL DEFAULT 0 CHECK(file_count >= 0);
ALTER TABLE backup_snapshots ADD COLUMN successful_count INTEGER NOT NULL DEFAULT 0 CHECK(successful_count >= 0);
ALTER TABLE backup_snapshots ADD COLUMN failed_count INTEGER NOT NULL DEFAULT 0 CHECK(failed_count >= 0);
ALTER TABLE backup_snapshots ADD COLUMN skipped_count INTEGER NOT NULL DEFAULT 0 CHECK(skipped_count >= 0);
ALTER TABLE backup_snapshots ADD COLUMN verified_bytes INTEGER NOT NULL DEFAULT 0 CHECK(verified_bytes >= 0);
ALTER TABLE backup_snapshots ADD COLUMN manifest_path TEXT;
ALTER TABLE backup_snapshots ADD COLUMN warnings_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE backup_snapshots ADD COLUMN local_integrity TEXT NOT NULL DEFAULT 'unchecked';
ALTER TABLE backup_snapshots ADD COLUMN local_verified_at INTEGER;
UPDATE backup_snapshots SET phase=CASE status WHEN 'complete' THEN 'completed' WHEN 'failed' THEN 'failed' WHEN 'cancelled' THEN 'cancelled' ELSE 'interrupted' END;
CREATE TABLE backup_jobs (
 id TEXT PRIMARY KEY NOT NULL,
 snapshot_id TEXT NOT NULL UNIQUE REFERENCES backup_snapshots(id),
 phase TEXT NOT NULL,
 copied_bytes INTEGER NOT NULL DEFAULT 0 CHECK(copied_bytes>=0),
 verified_bytes INTEGER NOT NULL DEFAULT 0 CHECK(verified_bytes>=0),
 error_code TEXT,
 updated_at INTEGER NOT NULL,
 stage_relative TEXT NOT NULL CHECK(stage_relative LIKE 'temp/backup-%' AND instr(stage_relative,'..')=0 AND instr(stage_relative,'\')=0 AND instr(stage_relative,':')=0)
) STRICT;
CREATE TABLE backup_files (
 id TEXT PRIMARY KEY NOT NULL,
 snapshot_id TEXT NOT NULL REFERENCES backup_snapshots(id),
 category TEXT NOT NULL,
 source_relative TEXT NOT NULL,
 local_relative TEXT NOT NULL,
 original_name TEXT NOT NULL,
 expected_size INTEGER CHECK(expected_size IS NULL OR expected_size>=0),
 actual_bytes INTEGER NOT NULL DEFAULT 0 CHECK(actual_bytes>=0),
 sha256 TEXT,
 local_verification TEXT NOT NULL DEFAULT 'pending',
 source_verification TEXT NOT NULL DEFAULT 'not_requested',
 size_validation TEXT NOT NULL DEFAULT 'unknown',
 state TEXT NOT NULL DEFAULT 'pending',
 error_code TEXT,
 UNIQUE(snapshot_id,local_relative)
) STRICT;
CREATE INDEX backup_files_snapshot ON backup_files(snapshot_id,id);
