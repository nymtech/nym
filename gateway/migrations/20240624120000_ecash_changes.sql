/*
 * Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */

ALTER TABLE available_bandwidth
RENAME COLUMN freepass_expiration TO expiration;

DROP TABLE spent_credential;

-- we need the id field to prevent data duplication
CREATE TABLE shared_keys_tmp (
    id                                       INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    client_address_bs58                      TEXT NOT NULL UNIQUE,
    derived_aes128_ctr_blake3_hmac_keys_bs58 TEXT NOT NULL
);

INSERT INTO shared_keys_tmp (client_address_bs58, derived_aes128_ctr_blake3_hmac_keys_bs58)
SELECT * FROM shared_keys;

-- ideally this table would be called "clients" but I don't want to cause too many breaking changes
DROP TABLE shared_keys;
ALTER TABLE shared_keys_tmp RENAME TO shared_keys;

CREATE TABLE available_bandwidth_tmp (
    client_id INTEGER NOT NULL PRIMARY KEY REFERENCES shared_keys(id),
    available INTEGER NOT NULL,
    expiration TIMESTAMP WITHOUT TIME ZONE
);

INSERT INTO available_bandwidth_tmp (client_id, available, expiration)
SELECT t1.id as client_id, t2.available, t2.expiration 
    FROM shared_keys as t1
    JOIN available_bandwidth as t2
    ON t1.client_address_bs58 = t2.client_address_bs58;

DROP TABLE available_bandwidth;
ALTER TABLE available_bandwidth_tmp RENAME TO available_bandwidth;

CREATE TABLE ecash_signer (
    epoch_id     INTEGER NOT NULL,

--    unique id assigned by the DKG contract. it does not change between epochs
    signer_id    INTEGER NOT NULL
);

CREATE TABLE received_ticket (
    id                  INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    client_id           INTEGER NOT NULL REFERENCES shared_keys(id),
    received_at         TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    rejected            BOOLEAN
);

CREATE INDEX received_ticket_index ON received_ticket (received_at);

-- received tickets that are in the process of verifying
CREATE TABLE ticket_data (
    ticket_id           INTEGER NOT NULL PRIMARY KEY REFERENCES received_ticket(id),

    -- serial_number, alongside the entire row, will get purged after redemption is complete
    serial_number       BLOB NOT NULL UNIQUE,

    --    data will get purged after 80% of signers verifies it
    data                BLOB
);


-- result of a verification from a single signer (API)
CREATE TABLE ticket_verification (
    ticket_id           INTEGER NOT NULL REFERENCES received_ticket(id),
    signer_id           INTEGER NOT NULL,
    verified_at         TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    accepted            BOOLEAN NOT NULL,
    
    PRIMARY KEY (ticket_id, signer_id)
);

CREATE INDEX ticket_verification_index ON ticket_verification (ticket_id);

-- verified tickets that are yet to be redeemed
CREATE TABLE verified_tickets (
    ticket_id           INTEGER NOT NULL PRIMARY KEY REFERENCES received_ticket(id),
    proposal_id         INTEGER REFERENCES redemption_proposals(proposal_id)
);

CREATE INDEX verified_tickets_index ON verified_tickets (proposal_id);

CREATE TABLE redemption_proposals (
    proposal_id         INTEGER NOT NULL PRIMARY KEY,
    created_at          TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    resolved_at         TIMESTAMP WITHOUT TIME ZONE, -- either got executed or got rejected 
    rejected            BOOLEAN
);