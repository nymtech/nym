/*
 * Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

DROP TABLE coconut_credentials;
CREATE TABLE coconut_credentials
(
    id                  INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    voucher_value       TEXT    NOT NULL,
    voucher_info        TEXT    NOT NULL,
    serial_number       TEXT    NOT NULL,
    binding_number      TEXT    NOT NULL,
    signature           TEXT    NOT NULL UNIQUE,
    epoch_id            TEXT    NOT NULL,
    consumed            BOOLEAN NOT NULL
);