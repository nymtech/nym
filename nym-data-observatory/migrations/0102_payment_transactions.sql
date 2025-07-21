CREATE TABLE payments (
    id INTEGER PRIMARY KEY,
    transaction_hash TEXT NOT NULL UNIQUE,
    sender_address TEXT NOT NULL,
    receiver_address TEXT NOT NULL,
    amount double precision NOT NULL,
    timestamp bigint NOT NULL,
    height bigint NOT NULL,
    memo TEXT
);
