/*
 * Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */
 
 
CREATE TABLE bloomfilter_parameters (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    num_hashes INTEGER NOT NULL,
    bitmap_size INTEGER NOT NULL,
    
    sip0_key0 BLOB NOT NULL,
    sip0_key1 BLOB NOT NULL,
    
    sip1_key0 BLOB NOT NULL,
    sip1_key1 BLOB NOT NULL
);

-- table containing partial bloomfilters produced from tickets spent on particular date
-- the 'current' bloomfilter is always OR(last_30)
CREATE TABLE partial_bloomfilter (
    date DATE NOT NULL UNIQUE PRIMARY KEY,
    parameters INTEGER NOT NULL REFERENCES bloomfilter_parameters(id),
    bitmap BLOB NOT NULL
);

CREATE TABLE master_verification_key (
    epoch_id INTEGER PRIMARY KEY NOT NULL,
    serialised_key BLOB NOT NULL
);

CREATE TABLE global_coin_index_signatures (
    -- we can only have a single entry
    epoch_id INTEGER PRIMARY KEY NOT NULL,
    
    -- combined signatures for all indices
    serialised_signatures BLOB NOT NULL
);

CREATE TABLE partial_coin_index_signatures (
    -- we can only have a single entry
    epoch_id INTEGER PRIMARY KEY NOT NULL,
    
    serialised_signatures BLOB NOT NULL
);

CREATE TABLE global_expiration_date_signatures (
    expiration_date DATE NOT NULL UNIQUE PRIMARY KEY,
    
    epoch_id INTEGER NOT NULL,
    
    -- combined signatures for all tuples issued for given day
    serialised_signatures BLOB NOT NULL
);

CREATE TABLE partial_expiration_date_signatures (
    expiration_date DATE NOT NULL UNIQUE PRIMARY KEY,
    
    epoch_id INTEGER NOT NULL,
    
    serialised_signatures BLOB NOT NULL
);

 
DROP TABLE issued_credential;
 
-- particular **partial** ecash ticketbook issued in this epoch
CREATE TABLE issued_ticketbook
(
    id                              INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    epoch_id                        INTEGER NOT NULL,
    deposit_id                      INTEGER NOT NULL UNIQUE,
    partial_credential              BLOB NOT NULL,
    signature                       BLOB NOT NULL,
    joined_private_commitments      BLOB NOT NULL,
    expiration_date                 DATE NOT NULL,
    ticketbook_type_repr            INTEGER NOT NULL
);
 
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
    spending_date       DATE NOT NULL,
    verified_at         TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    gateway_id          INTEGER NOT NULL REFERENCES ticket_providers(id)
);

-- helper index for getting tickets verified by particular gateway
CREATE INDEX verified_tickets_index ON verified_tickets (gateway_id, verified_at desc);

-- helper index for getting all tickets with particular spending date for rebuilding the bloomfilters
CREATE INDEX verified_tickets_spending_index ON verified_tickets (spending_date);