-- Nullable resolved references leave existing exports/native imports intact.
ALTER TABLE messages ADD COLUMN native_reply_id TEXT REFERENCES messages(id) ON DELETE SET NULL;
CREATE INDEX messages_native_source ON messages(conversation_id,native_id);
