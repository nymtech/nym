/*
 * Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */
 
DROP TABLE coconut_credentials;

CREATE TABLE master_verification_key (
    epoch_id INTEGER PRIMARY KEY NOT NULL,
    
    serialised_key BLOB NOT NULL
);

CREATE TABLE coin_indices_signatures
(
    epoch_id INTEGER PRIMARY KEY NOT NULL,
    
    serialised_signatures BLOB NOT NULL
);

CREATE TABLE expiration_date_signatures (
    expiration_date DATE NOT NULL UNIQUE PRIMARY KEY,
    
    epoch_id INTEGER NOT NULL,
    
    -- combined signatures for all tuples issued for given day
    serialised_signatures BLOB NOT NULL
);


CREATE TABLE ecash_ticketbook
(
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    
    -- introduce a way for us to introduce breaking changes in serialization of data
    serialization_revision INTEGER NOT NULL,
    
    -- the type of the associated ticketbook   
    ticketbook_type TEXT NOT NULL,
    
    -- the actual crypto data of the ticketbook (wallet, keys, etc.)
    ticketbook_data BLOB NOT NULL UNIQUE,
    
    -- for each ticketbook we MUST have corresponding expiration date signatures
    expiration_date DATE NOT NULL REFERENCES expiration_date_signatures(expiration_date),
    
    -- for each ticketbook we MUST have corresponding coin index signatures
    epoch_id INTEGER NOT NULL REFERENCES coin_indices_signatures(epoch_id),
    
    -- the initial number of tickets the wallet has been created for
    total_tickets INTEGER NOT NULL,
    
    -- how many tickets have been used so far (the `l` value of the wallet)
    used_tickets INTEGER NOT NULL
);

-- data for ticketbooks that have an associated deposit, but failed to get issued
CREATE TABLE pending_issuance 
(
    deposit_id INTEGER NOT NULL PRIMARY KEY,
    
    -- introduce a way for us to introduce breaking changes in serialization of data
    serialization_revision INTEGER NOT NULL,
    
    pending_ticketbook_data BLOB NOT NULL UNIQUE,
    
    -- for each ticketbook we MUST have corresponding expiration date signatures
    expiration_date DATE NOT NULL REFERENCES expiration_date_signatures(expiration_date)
);