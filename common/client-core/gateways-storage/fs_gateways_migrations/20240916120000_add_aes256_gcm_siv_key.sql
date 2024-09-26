/*
 * Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

-- make aes128 key column nullable and add aes256 column
ALTER TABLE remote_gateway_details RENAME COLUMN derived_aes128_ctr_blake3_hmac_keys_bs58 TO derived_aes128_ctr_blake3_hmac_keys_bs58_old;
ALTER TABLE remote_gateway_details ADD COLUMN derived_aes128_ctr_blake3_hmac_keys_bs58 TEXT;
ALTER TABLE remote_gateway_details ADD COLUMN derived_aes256_gcm_siv_key BLOB;

UPDATE remote_gateway_details SET derived_aes128_ctr_blake3_hmac_keys_bs58 = derived_aes128_ctr_blake3_hmac_keys_bs58_old;

ALTER TABLE remote_gateway_details DROP COLUMN derived_aes128_ctr_blake3_hmac_keys_bs58_old;