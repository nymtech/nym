/*
 * Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

CREATE TABLE remote_gateway_details_temp
(
    gateway_id_bs58                          TEXT NOT NULL UNIQUE PRIMARY KEY REFERENCES registered_gateway (gateway_id_bs58),
    derived_aes256_gcm_siv_key               BLOB NOT NULL,
    gateway_details                          TEXT NOT NULL CHECK (json_valid(gateway_details)),
    expiration_timestamp                     DATETIME NOT NULL
);

-- keep none, the gateways listener URL does not contain the gateway details information
-- INSERT INTO remote_gateway_details_temp SELECT gateway_id_bs58, derived_aes256_gcm_siv_key, gateway_listener, NULL, datetime(0, 'unixepoch') FROM remote_gateway_details WHERE derived_aes256_gcm_siv_key IS NOT NULL;

DROP TABLE remote_gateway_details;
ALTER TABLE remote_gateway_details_temp RENAME TO remote_gateway_details;

-- delete registrations with no key
DELETE FROM registered_gateway WHERE gateway_id_bs58 NOT IN ( SELECT gateway_id_bs58 FROM remote_gateway_details);
