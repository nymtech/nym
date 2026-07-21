/*
 * Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */
 

-- data for ticketbooks that have an associated deposit, but failed to get issued
CREATE TABLE pending_issuance 
(
    deposit_id INTEGER NOT NULL PRIMARY KEY,
    
    -- introduce a way for us to introduce breaking changes in serialization of data
    serialization_revision INTEGER NOT NULL,
    
    pending_ticketbook_data BLOB NOT NULL UNIQUE,
    
    -- for each ticketbook we MUST have corresponding expiration date signatures
    expiration_date DATE NOT NULL
);