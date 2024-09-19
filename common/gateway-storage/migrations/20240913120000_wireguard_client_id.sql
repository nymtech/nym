/*
 * Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

ALTER TABLE wireguard_peer
ADD COLUMN client_id INTEGER REFERENCES clients(id) DEFAULT NULL;

ALTER TABLE wireguard_peer
DROP COLUMN suspended;
