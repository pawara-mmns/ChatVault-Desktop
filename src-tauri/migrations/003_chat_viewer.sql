-- Preserve existing/imported sequence. Legacy records without an order follow
-- their existing rowid order after any explicitly sequenced records.
WITH legacy AS MATERIALIZED (
 SELECT id, COALESCE((SELECT max(sequence_number) FROM messages m2
   WHERE m2.conversation_id=m.conversation_id),-1)
   + row_number() OVER (PARTITION BY conversation_id ORDER BY rowid) AS ordinal
 FROM messages m WHERE sequence_number IS NULL
)
UPDATE messages SET sequence_number=(SELECT ordinal FROM legacy WHERE legacy.id=messages.id)
WHERE sequence_number IS NULL;

CREATE INDEX messages_conversation_sequence ON messages(conversation_id,sequence_number,id);

-- Trigrams preserve combining marks and support literal substrings, including
-- Sinhala. NFC/lowercase is applied by the innocuous Rust scalar function.
-- Immutable imports maintain this index explicitly in the SAME transaction as
-- message inserts. No unsafe virtual-table trigger or trusted_schema relaxation.
CREATE VIRTUAL TABLE message_search USING fts5(body,sender,tokenize='trigram case_sensitive 1');
INSERT INTO message_search(rowid,body,sender)
SELECT rowid,chatvault_fold(message_text),chatvault_fold(COALESCE(sender,'')) FROM messages;
