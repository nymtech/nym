CREATE TABLE IF NOT EXISTS transactions (
    id INTEGER PRIMARY KEY,
    tx_hash TEXT NOT NULL,
    height BIGINT NOT NULL,
    message_index BIGINT NOT NULL,
    sender TEXT NOT NULL,
    recipient TEXT NOT NULL,
    amount TEXT NOT NULL,
    memo TEXT,
    created_at DATE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(tx_hash, message_index)
);