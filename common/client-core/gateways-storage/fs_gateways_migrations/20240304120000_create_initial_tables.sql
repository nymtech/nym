/*
 * Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

CREATE TABLE active_gateway
(
    id                     INTEGER PRIMARY KEY CHECK (id = 0),
    active_gateway_id_bs58 TEXT REFERENCES registered_gateway (gateway_id_bs58)
);

CREATE TABLE registered_gateway
(
    gateway_id_bs58        TEXT                                                NOT NULL UNIQUE PRIMARY KEY,
    registration_timestamp TIMESTAMP WITHOUT TIME ZONE                         NOT NULL,
    gateway_type           TEXT CHECK ( gateway_type IN ('remote', 'custom') ) NOT NULL DEFAULT 'remote'
);

-- TODO: perhaps keep additional metadata such as bandwidth, credential usage, etc


CREATE TABLE remote_gateway_details
(
    gateway_id_bs58                          TEXT NOT NULL UNIQUE PRIMARY KEY REFERENCES registered_gateway (gateway_id_bs58),
    derived_aes128_ctr_blake3_hmac_keys_bs58 TEXT NOT NULL,
    gateway_owner_address                    TEXT NOT NULL,
    gateway_listener                         TEXT NOT NULL
);

CREATE TABLE custom_gateway_details
(
    gateway_id_bs58 TEXT NOT NULL UNIQUE PRIMARY KEY REFERENCES registered_gateway (gateway_id_bs58),
    data            BLOB
);

