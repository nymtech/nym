/*
 * Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

CREATE TABLE remote_gateway_details_temp
(
    gateway_id_bs58                          TEXT NOT NULL UNIQUE PRIMARY KEY REFERENCES registered_gateway (gateway_id_bs58),
    derived_aes128_ctr_blake3_hmac_keys_bs58 TEXT NOT NULL,
    gateway_owner_address                    TEXT,
    gateway_listener                         TEXT NOT NULL
);

INSERT INTO remote_gateway_details_temp SELECT gateway_id_bs58, derived_aes128_ctr_blake3_hmac_keys_bs58, gateway_owner_address, gateway_listener FROM remote_gateway_details;

DROP TABLE remote_gateway_details;
ALTER TABLE remote_gateway_details_temp RENAME TO remote_gateway_details;