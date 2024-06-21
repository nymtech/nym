/*
 * Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */

DROP TABLE spent_credentials;

CREATE TABLE ticket_providers
(
    id                      INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    gateway_address         TEXT NOT NULL UNIQUE,
    last_batch_verification TIMESTAMP WITHOUT TIME ZONE
);

CREATE TABLE verified_tickets
(
    id                  INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    ticket_data         BLOB NOT NULL,
    serial_number       BLOB NOT NULL UNIQUE,
    verified_at         TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    gateway_id          INTEGER NOT NULL REFERENCES ticket_providers(id)
);

CREATE INDEX verified_tickets_index ON verified_tickets (gateway_id, verified_at desc);