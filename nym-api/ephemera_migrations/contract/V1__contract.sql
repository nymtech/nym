CREATE TABLE contract_mixnode_reward
(
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    mix_id             INTEGER NOT NULL,
    epoch              INTEGER NOT NULL,
    nym_api_id         INTEGER NOT NULL,
    reliability        INTEGER NOT NULL,
    timestamp          INTEGER NOT NULL,
    UNIQUE (mix_id, epoch)
);

CREATE TABLE epoch_info
(
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    epoch_id           INTEGER NOT NULL,
    start_time         INTEGER NOT NULL,
    duration           INTEGER NOT NULL,
    UNIQUE (epoch_id)
);