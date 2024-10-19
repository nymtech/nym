/*
 * Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */

-- make aes128 key column nullable and add aes256 column
ALTER TABLE shared_keys RENAME COLUMN derived_aes128_ctr_blake3_hmac_keys_bs58 TO derived_aes128_ctr_blake3_hmac_keys_bs58_old;
ALTER TABLE shared_keys ADD COLUMN derived_aes128_ctr_blake3_hmac_keys_bs58 TEXT;
ALTER TABLE shared_keys ADD COLUMN derived_aes256_gcm_siv_key BLOB;

UPDATE shared_keys SET derived_aes128_ctr_blake3_hmac_keys_bs58 = derived_aes128_ctr_blake3_hmac_keys_bs58_old;

ALTER TABLE shared_keys DROP COLUMN derived_aes128_ctr_blake3_hmac_keys_bs58_old;
