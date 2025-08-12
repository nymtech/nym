/*
 * Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

-- 1. add temporary `epoch_id` column
ALTER TABLE pending_issuance
    ADD COLUMN epoch_id INTEGER;

-- 2. populate the value
UPDATE pending_issuance
SET epoch_id = (SELECT epoch_id
                FROM expiration_date_signatures
                WHERE expiration_date_signatures.expiration_date = pending_issuance.expiration_date);

-- 3. create new expiration_date_signatures table (with changed constraints)
CREATE TABLE expiration_date_signatures_new
(
    expiration_date        DATE    NOT NULL,

    epoch_id               INTEGER NOT NULL,

    serialization_revision INTEGER NOT NULL,

    -- combined signatures for all tuples issued for given day
    serialised_signatures  BLOB    NOT NULL,

    PRIMARY KEY (epoch_id, expiration_date)
);

-- 4. migrate the data
INSERT INTO expiration_date_signatures_new (expiration_date, epoch_id, serialization_revision, serialised_signatures)
SELECT expiration_date, epoch_id, serialization_revision, serialised_signatures
FROM expiration_date_signatures;

-- 5. drop and recreate the table references (due to new FK)

-- 5.1.
-- (data for ticketbooks that have an associated deposit, but failed to get issued)
CREATE TABLE pending_issuance_new
(
    deposit_id              INTEGER NOT NULL PRIMARY KEY,

    -- introduce a way for us to introduce breaking changes in serialization of data
    serialization_revision  INTEGER NOT NULL,

    pending_ticketbook_data BLOB    NOT NULL UNIQUE,

    -- for each ticketbook we MUST have corresponding expiration date signatures
    expiration_date         DATE    NOT NULL,
    epoch_id                INTEGER NOT NULL,

    -- for each ticketbook we MUST have corresponding expiration date signatures
    FOREIGN KEY (epoch_id, expiration_date) REFERENCES expiration_date_signatures_new (epoch_id, expiration_date)
);

INSERT INTO pending_issuance_new (deposit_id, serialization_revision, pending_ticketbook_data, expiration_date,
                                  epoch_id)
SELECT deposit_id, serialization_revision, pending_ticketbook_data, expiration_date, epoch_id
FROM pending_issuance;


-- 5.2.
CREATE TABLE ecash_ticketbook_new
(
    id                     INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,

    -- introduce a way for us to introduce breaking changes in serialization of data
    serialization_revision INTEGER NOT NULL,

    -- the type of the associated ticketbook
    ticketbook_type        TEXT    NOT NULL,

    -- the actual crypto data of the ticketbook (wallet, keys, etc.)
    ticketbook_data        BLOB    NOT NULL UNIQUE,

    -- for each ticketbook we MUST have corresponding expiration date signatures
    expiration_date        DATE    NOT NULL,

    -- for each ticketbook we MUST have corresponding coin index signatures
    epoch_id               INTEGER NOT NULL,

    -- the initial number of tickets the wallet has been created for
    total_tickets          INTEGER NOT NULL,

    -- how many tickets have been used so far (the `l` value of the wallet)
    used_tickets           INTEGER NOT NULL,


    -- FOREIGN KEYS:

    -- for each ticketbook we MUST have corresponding coin index signatures
    FOREIGN KEY (epoch_id) REFERENCES coin_indices_signatures (epoch_id),

    -- for each ticketbook we MUST have corresponding expiration date signatures
    FOREIGN KEY (expiration_date, epoch_id) REFERENCES expiration_date_signatures_new (expiration_date, epoch_id)
);

INSERT INTO ecash_ticketbook_new (id, serialization_revision, ticketbook_type, ticketbook_data, expiration_date,
                                  epoch_id, total_tickets, used_tickets)
SELECT id,
       serialization_revision,
       ticketbook_type,
       ticketbook_data,
       expiration_date,
       epoch_id,
       total_tickets,
       used_tickets
FROM ecash_ticketbook;

-- 6. finally swap out the old tables
-- drop old tables
DROP TABLE expiration_date_signatures;
DROP TABLE pending_issuance;
DROP TABLE ecash_ticketbook;

-- rename new tables
ALTER TABLE expiration_date_signatures_new
    RENAME TO expiration_date_signatures;
ALTER TABLE pending_issuance_new
    RENAME TO pending_issuance;
ALTER TABLE ecash_ticketbook_new
    RENAME TO ecash_ticketbook;