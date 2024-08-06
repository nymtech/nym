/*
 * Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

CREATE TABLE spent_credential
(
    blinded_serial_number_bs58 TEXT    NOT NULL PRIMARY KEY UNIQUE,
    was_freepass               BOOLEAN NOT NULL,
    client_address_bs58        TEXT    NOT NULL REFERENCES shared_keys (client_address_bs58)
);