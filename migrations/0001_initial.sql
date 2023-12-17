-- sqlite.

-- This table is used to store the key-value pairs.
--
-- `next_index` is used to determine the next account index to use.
-- `last_block` that was observed.
CREATE TABLE kv (
    key TEXT NOT NULL,
    value TEXT
);

INSERT INTO kv (key, value) VALUES ('next_index', '0');
INSERT INTO kv (key, value) VALUES ('last_block', NULL);

-- This table is used to store the transactions to send.
CREATE TABLE txns (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    -- The binary data of the transaction. Could be directly submitted to the RPC.
    extrinsic_data BLOB NOT NULL
);
