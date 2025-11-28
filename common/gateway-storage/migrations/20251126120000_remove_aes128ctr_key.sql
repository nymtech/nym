/*
 * Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */


-- make aes256gcm column non-nullable and drop any clients that still use the legacy keys
CREATE TABLE shared_keys_tmp
(
    client_id                  INTEGER NOT NULL PRIMARY KEY REFERENCES clients (id),
    client_address_bs58        TEXT    NOT NULL UNIQUE,
    derived_aes256_gcm_siv_key BLOB    NOT NULL
);

INSERT INTO shared_keys_tmp (client_id, client_address_bs58, derived_aes256_gcm_siv_key)
SELECT client_id, client_address_bs58, derived_aes256_gcm_siv_key
FROM shared_keys
WHERE derived_aes256_gcm_siv_key IS NOT NULL;

DROP TABLE shared_keys;
ALTER TABLE shared_keys_tmp
    RENAME TO shared_keys;