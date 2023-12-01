/*
 * Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

CREATE TABLE shared_keys
(
    client_address_bs58                      TEXT NOT NULL PRIMARY KEY UNIQUE,
    derived_aes128_ctr_blake3_hmac_keys_bs58 TEXT NOT NULL
);

CREATE TABLE message_store
(
    id                  INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    client_address_bs58 TEXT    NOT NULL,
    content             BLOB    NOT NULL
);

CREATE TABLE available_bandwidth
(
    client_address_bs58 TEXT    NOT NULL PRIMARY KEY UNIQUE,
    available           INTEGER NOT NULL
);

CREATE INDEX `message_store_index` ON `message_store` (`client_address_bs58`, `content`);