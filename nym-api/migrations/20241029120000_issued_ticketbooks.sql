/*
 * Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

-- remove old tables as they don't have the required information
DROP TABLE epoch_credentials;
DROP TABLE issued_ticketbook;


CREATE TABLE issued_ticketbook
(
    deposit_id                 INTEGER NOT NULL PRIMARY KEY,
    dkg_epoch_id               INTEGER NOT NULL,
    blinded_partial_credential BLOB    NOT NULL,
    joined_private_commitments BLOB    NOT NULL,
    expiration_date            DATE    NOT NULL,
    ticketbook_type_repr       INTEGER NOT NULL,

    -- hash on the whole data as in what has been inserted into the merkle tree
    merkle_leaf                BLOB    NOT NULL
);

-- helper index for getting tickets issued with particular expiration date for easier proof construction
CREATE INDEX issued_ticketbook_date ON issued_ticketbook (expiration_date);