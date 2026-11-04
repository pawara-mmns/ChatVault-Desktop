-- All timestamps are UTC Unix epoch milliseconds. Paths are vault-relative, using '/'.
CREATE TABLE app_settings (
    key TEXT PRIMARY KEY NOT NULL CHECK(length(key) > 0),
    value TEXT NOT NULL,
    updated_at INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)) CHECK(updated_at >= 0)
) STRICT;

CREATE TABLE devices (
    id TEXT PRIMARY KEY NOT NULL CHECK(length(id) > 0),
    display_name TEXT NOT NULL CHECK(length(display_name) > 0),
    platform TEXT NOT NULL CHECK(platform = 'android'),
    last_connected_at INTEGER CHECK(last_connected_at IS NULL OR last_connected_at >= 0),
    created_at INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)) CHECK(created_at >= 0)
) STRICT;

CREATE TABLE backup_snapshots (
    id TEXT PRIMARY KEY NOT NULL CHECK(length(id) > 0),
    device_id TEXT REFERENCES devices(id) ON DELETE SET NULL,
    status TEXT NOT NULL CHECK(status IN ('pending', 'in-progress', 'complete', 'failed', 'cancelled')),
    storage_path TEXT NOT NULL CHECK(storage_path LIKE 'backups/%' AND instr(storage_path, '..') = 0 AND instr(storage_path, '\') = 0 AND instr(storage_path, ':') = 0),
    total_bytes INTEGER NOT NULL DEFAULT 0 CHECK(total_bytes >= 0),
    created_at INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)) CHECK(created_at >= 0),
    completed_at INTEGER CHECK(completed_at IS NULL OR completed_at >= created_at)
) STRICT;

CREATE TABLE archives (
    id TEXT PRIMARY KEY NOT NULL CHECK(length(id) > 0),
    title TEXT NOT NULL CHECK(length(title) > 0),
    source_type TEXT NOT NULL CHECK(source_type = 'chat-export'),
    storage_path TEXT NOT NULL CHECK(storage_path LIKE 'archives/%' AND instr(storage_path, '..') = 0 AND instr(storage_path, '\') = 0 AND instr(storage_path, ':') = 0),
    created_at INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)) CHECK(created_at >= 0)
) STRICT;

CREATE TABLE conversations (
    id TEXT PRIMARY KEY NOT NULL CHECK(length(id) > 0),
    archive_id TEXT NOT NULL REFERENCES archives(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)) CHECK(created_at >= 0)
) STRICT;

CREATE TABLE messages (
    id TEXT PRIMARY KEY NOT NULL CHECK(length(id) > 0),
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    sender TEXT,
    message_text TEXT NOT NULL,
    timestamp INTEGER CHECK(timestamp IS NULL OR timestamp >= 0)
) STRICT;

CREATE TABLE attachments (
    id TEXT PRIMARY KEY NOT NULL CHECK(length(id) > 0),
    message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    media_type TEXT,
    storage_path TEXT NOT NULL CHECK(storage_path LIKE 'media/%' AND instr(storage_path, '..') = 0 AND instr(storage_path, '\') = 0 AND instr(storage_path, ':') = 0),
    file_size INTEGER CHECK(file_size IS NULL OR file_size >= 0)
) STRICT;

CREATE INDEX backup_snapshots_device ON backup_snapshots(device_id);
CREATE INDEX backup_snapshots_created ON backup_snapshots(created_at DESC);
CREATE INDEX archives_created ON archives(created_at DESC);
CREATE INDEX conversations_archive ON conversations(archive_id);
CREATE INDEX messages_conversation_timestamp ON messages(conversation_id, timestamp, id);
CREATE INDEX attachments_message ON attachments(message_id);
