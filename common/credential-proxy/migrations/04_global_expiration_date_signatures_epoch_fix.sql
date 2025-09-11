/*
 * Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */

DROP TABLE global_expiration_date_signatures;

CREATE TABLE global_expiration_date_signatures
(
    expiration_date        DATE    NOT NULL,
    epoch_id               INTEGER NOT NULL,
    serialization_revision INTEGER NOT NULL,

    -- combined signatures for all tuples issued for given day
    serialised_signatures  BLOB    NOT NULL,

    PRIMARY KEY (epoch_id, expiration_date)
)