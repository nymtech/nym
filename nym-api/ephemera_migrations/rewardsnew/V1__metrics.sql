CREATE TABLE mixnode_status
(
    mix_id             INTEGER NOT NULL,
    reliability        INTEGER NOT NULL,
    timestamp          INTEGER NOT NULL,
    UNIQUE (mix_id, timestamp)
);

CREATE TABLE rewarding_report
(
    epoch_id                 INTEGER NOT NULL,
    eligible_mixnodes        INTEGER NOT NULL,
    timestamp                INTEGER NOT NULL,
    UNIQUE (epoch_id)
);

CREATE TABLE epoch_blocks
(
    epoch_id             INTEGER NOT NULL,
    block_id             INTEGER NOT NULL,
    timestamp            INTEGER NOT NULL,
    UNIQUE (epoch_id, block_id)
);