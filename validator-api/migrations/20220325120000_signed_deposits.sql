CREATE TABLE signed_deposit
(
    id                         INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    tx_hash                    VARCHAR NOT NULL UNIQUE,
    blinded_signature_response VARCHAR NOT NULL
);