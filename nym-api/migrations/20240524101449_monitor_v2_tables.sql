CREATE TABLE mixnode_details_v2
(
    id       INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    mix_id   INTEGER NOT NULL UNIQUE,
    owner    VARCHAR NOT NULL,
    identity_key VARCHAR NOT NULL
);

CREATE TABLE mixnode_status_v2
(
    mixnode_details_id INTEGER NOT NULL,
    reliability        INTEGER NOT NULL,
    timestamp          INTEGER NOT NULL
);
