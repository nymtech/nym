/*
 * Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

CREATE TABLE coconut_credentials
(
    id                  INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    voucher_value       TEXT    NOT NULL,
    voucher_info        TEXT    NOT NULL,
    serial_number       TEXT    NOT NULL,
    binding_number      TEXT    NOT NULL,
    signature           TEXT    NOT NULL UNIQUE
);

CREATE TABLE erc20_credentials
(
    id                  INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    public_key          TEXT    NOT NULL,
    private_key         TEXT    NOT NULL,
    consumed            BOOLEAN NOT NULL
);