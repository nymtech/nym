/*
 * Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */

-- make aes256gcm column non-nullable and drop any gateways that still use the legacy keys
-- (since they'd be unusable after this change)
CREATE TABLE remote_gateway_details_tmp
(
    gateway_id_bs58            TEXT NOT NULL UNIQUE PRIMARY KEY REFERENCES registered_gateway (gateway_id_bs58),
    derived_aes256_gcm_siv_key BLOB NOT NULL,
    gateway_owner_address      TEXT,
    gateway_listener           TEXT NOT NULL
);

INSERT INTO remote_gateway_details_tmp (gateway_id_bs58, derived_aes256_gcm_siv_key, gateway_owner_address,
                                        gateway_listener)
SELECT gateway_id_bs58, derived_aes256_gcm_siv_key, gateway_owner_address, gateway_listener
FROM remote_gateway_details
WHERE derived_aes256_gcm_siv_key IS NOT NULL;

DROP TABLE remote_gateway_details;
ALTER TABLE remote_gateway_details_tmp
    RENAME TO remote_gateway_details;