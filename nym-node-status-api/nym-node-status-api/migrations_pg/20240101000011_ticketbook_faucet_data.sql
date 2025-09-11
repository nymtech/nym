/*
 * Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */


CREATE TABLE master_verification_key
(
    epoch_id               INTEGER PRIMARY KEY NOT NULL,
    serialization_revision SMALLINT            NOT NULL,
    serialised_key         BYTEA               NOT NULL
);

CREATE TABLE global_coin_index_signatures
(
    -- we can only have a single entry
    epoch_id               INTEGER PRIMARY KEY NOT NULL,
    serialization_revision SMALLINT            NOT NULL,

    -- combined signatures for all indices
    serialised_signatures  BYTEA               NOT NULL
);


CREATE TABLE global_expiration_date_signatures
(
    expiration_date        DATE     NOT NULL,
    epoch_id               INTEGER  NOT NULL,
    serialization_revision SMALLINT NOT NULL,

    -- combined signatures for all tuples issued for given day
    serialised_signatures  BYTEA    NOT NULL,

    PRIMARY KEY (epoch_id, expiration_date)
);

CREATE TABLE ticketbook_deposit
(
    deposit_id                  INTEGER PRIMARY KEY         NOT NULL,
    deposit_tx_hash             TEXT                        NOT NULL,
    requested_on                TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    deposit_amount              TEXT                        NOT NULL,
    client_pubkey               BYTEA                       NOT NULL,
    ed25519_deposit_private_key BYTEA                       NOT NULL
);


-- CREATE TABLE pending_issuance
-- (
--     deposit_id              INTEGER NOT NULL PRIMARY KEY,
--
--     -- introduce a way for us to introduce breaking changes in serialization of data
--     serialization_revision  INTEGER NOT NULL,
--
--     pending_ticketbook_data BYTEA    NOT NULL UNIQUE,
--
--     -- for each ticketbook we MUST have corresponding expiration date signatures
--     expiration_date         DATE    NOT NULL,
--     epoch_id                INTEGER NOT NULL,
--
--     -- for each ticketbook we MUST have corresponding expiration date signatures
--     FOREIGN KEY (epoch_id, expiration_date) REFERENCES global_expiration_date_signatures (epoch_id, expiration_date)
-- );

CREATE TABLE ecash_ticketbook
(
    id                     SERIAL   NOT NULL PRIMARY KEY,

    -- introduce a way for us to introduce breaking changes in serialization of data
    serialization_revision SMALLINT NOT NULL,

    -- the type of the associated ticketbook
    ticketbook_type        TEXT     NOT NULL,

    -- the actual crypto data of the ticketbook (wallet, keys, etc.)
    ticketbook_data        BYTEA    NOT NULL UNIQUE,

    -- for each ticketbook we MUST have corresponding expiration date signatures
    expiration_date        DATE     NOT NULL,

    -- for each ticketbook we MUST have corresponding coin index signatures
    epoch_id               INTEGER  NOT NULL,

    -- the initial number of tickets the wallet has been created for
    total_tickets          INTEGER  NOT NULL,

    -- how many tickets have been used so far (the `l` value of the wallet)
    used_tickets           INTEGER  NOT NULL,


    -- FOREIGN KEYS:

    -- for each ticketbook we MUST have corresponding coin index signatures
    FOREIGN KEY (epoch_id) REFERENCES global_coin_index_signatures (epoch_id),

    -- for each ticketbook we MUST have corresponding expiration date signatures
    FOREIGN KEY (expiration_date, epoch_id) REFERENCES global_expiration_date_signatures (expiration_date, epoch_id)
);