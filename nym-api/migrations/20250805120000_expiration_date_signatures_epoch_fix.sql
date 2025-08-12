/*
 * Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */

-- Change performed in this migration:
-- remove PK on expiration_date and instead use composite (epoch_id, expiration_date) PK


CREATE TABLE global_expiration_date_signatures_new
(
    expiration_date       DATE    NOT NULL,

    epoch_id              INTEGER NOT NULL,

    -- combined signatures for all tuples issued for given day
    serialised_signatures BLOB    NOT NULL,

    PRIMARY KEY (epoch_id, expiration_date)
);

CREATE TABLE partial_expiration_date_signatures_new
(
    expiration_date       DATE    NOT NULL,

    epoch_id              INTEGER NOT NULL,

    serialised_signatures BLOB    NOT NULL,

    PRIMARY KEY (epoch_id, expiration_date)
);

-- global
INSERT INTO global_expiration_date_signatures_new
SELECT *
FROM global_expiration_date_signatures;

DROP TABLE global_expiration_date_signatures;

ALTER TABLE global_expiration_date_signatures_new
    RENAME TO global_expiration_date_signatures;

-- partial
INSERT INTO partial_expiration_date_signatures_new
SELECT *
FROM partial_expiration_date_signatures;

DROP TABLE partial_expiration_date_signatures;

ALTER TABLE partial_expiration_date_signatures_new
    RENAME TO partial_expiration_date_signatures;