-- Explicitly consented notes only; no hardware ID, serial, path, or live connection state.
-- The original Android-only devices/backup FK records remain untouched.
CREATE TABLE portable_device_notes (
    id TEXT PRIMARY KEY NOT NULL CHECK(length(id) = 36),
    display_name TEXT NOT NULL CHECK(length(display_name) BETWEEN 1 AND 160),
    category TEXT CHECK(category IS NULL OR length(category) <= 160),
    last_connected_at INTEGER NOT NULL CHECK(last_connected_at >= 0),
    preferred INTEGER NOT NULL DEFAULT 0 CHECK(preferred IN (0,1))
) STRICT;
