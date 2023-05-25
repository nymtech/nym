CREATE TABLE IF NOT EXISTS blocks (
    id          INTEGER      NOT NULL PRIMARY KEY AUTOINCREMENT,
    block_hash  TEXT         NOT NULL UNIQUE,
    height      TEXT         NOT NULL UNIQUE,
    block       BLOB         NOT NULL
);

CREATE TABLE IF NOT EXISTS block_certificates (
    id              INTEGER      NOT NULL PRIMARY KEY AUTOINCREMENT,
    block_hash      TEXT         NOT NULL UNIQUE,
    certificates    BLOB         NOT NULL
);

CREATE TABLE IF NOT EXISTS block_broadcast_group (
    id              INTEGER      NOT NULL PRIMARY KEY AUTOINCREMENT,
    block_hash      TEXT         NOT NULL UNIQUE,
    members         BLOB         NOT NULL
);

CREATE TABLE IF NOT EXISTS block_merkle_tree (
    id              INTEGER      NOT NULL PRIMARY KEY AUTOINCREMENT,
    block_hash      TEXT         NOT NULL UNIQUE,
    merkle_tree     BLOB         NOT NULL
);