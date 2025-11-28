/*
 * Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

CREATE TABLE remote_gateway_details_temp
(
    gateway_id_bs58                          TEXT NOT NULL UNIQUE PRIMARY KEY REFERENCES registered_gateway (gateway_id_bs58),
    gateway_listener                         TEXT NOT NULL,
    fallback_listener                        TEXT,
    expiration_timestamp                     TIMESTAMP WITHOUT TIME ZONE                          
);

CREATE TABLE remote_gateway_shared_keys
(
    gateway_id_bs58                          TEXT NOT NULL UNIQUE PRIMARY KEY REFERENCES registered_gateway (gateway_id_bs58),
    derived_aes256_gcm_siv_key               BLOB NOT NULL
);

INSERT INTO remote_gateway_shared_keys SELECT gateway_id_bs58, derived_aes256_gcm_siv_key FROM remote_gateway_details;

DROP TABLE remote_gateway_details;
ALTER TABLE remote_gateway_details_temp RENAME TO remote_gateway_details;



