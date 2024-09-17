/*
 * Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

ALTER TABLE master_verification_key
    ADD COLUMN serialization_revision INTEGER NOT NULL default 1;

ALTER TABLE coin_indices_signatures
    ADD COLUMN serialization_revision INTEGER NOT NULL default 1;

ALTER TABLE expiration_date_signatures
    ADD COLUMN serialization_revision INTEGER NOT NULL default 1;
