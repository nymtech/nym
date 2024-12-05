CREATE TABLE payments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    transaction_hash TEXT NOT NULL UNIQUE,
    sender_address TEXT NOT NULL,
    receiver_address TEXT NOT NULL,
    amount REAL NOT NULL,
    timestamp INTEGER NOT NULL,
    height INTEGER NOT NULL,
    memo TEXT
);
