DROP TABLE IF EXISTS mixnode_details_v2;

CREATE TABLE mixnode_details_v2
(
    id       INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    node_id   INTEGER NOT NULL UNIQUE,
    identity_key VARCHAR NOT NULL
);

DROP TABLE IF EXISTS gateway_details_v2;

CREATE TABLE gateway_details_v2
(
    id      INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    node_id  INTEGER NOT NULL UNIQUE,
    identity VARCHAR NOT NULL UNIQUE
);


