/*
 * Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

CREATE TABLE clients (
   id              INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
   client_type     TEXT NOT NULL CHECK(client_type IN ('entry mixnet', 'entry wireguard', 'exit wireguard'))
);

INSERT INTO clients (id, client_type)
SELECT id, 'entry mixnet'
FROM shared_keys;

CREATE TABLE shared_keys_tmp (
   client_id                                INTEGER NOT NULL PRIMARY KEY REFERENCES clients(id),
   client_address_bs58                      TEXT NOT NULL UNIQUE,
   derived_aes128_ctr_blake3_hmac_keys_bs58 TEXT NOT NULL
);

INSERT INTO shared_keys_tmp (client_id, client_address_bs58, derived_aes128_ctr_blake3_hmac_keys_bs58)
SELECT id as client_id, client_address_bs58, derived_aes128_ctr_blake3_hmac_keys_bs58 FROM shared_keys;

CREATE TABLE available_bandwidth_tmp (
   client_id INTEGER NOT NULL PRIMARY KEY REFERENCES clients(id),
   available INTEGER NOT NULL,
   expiration TIMESTAMP WITHOUT TIME ZONE
);

INSERT INTO available_bandwidth_tmp
SELECT * FROM available_bandwidth;

DROP TABLE available_bandwidth;
ALTER TABLE available_bandwidth_tmp RENAME TO available_bandwidth;

CREATE TABLE received_ticket_tmp (
   id                  INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
   client_id           INTEGER NOT NULL REFERENCES clients(id),
   received_at         TIMESTAMP WITHOUT TIME ZONE NOT NULL,
   rejected            BOOLEAN
);

INSERT INTO received_ticket_tmp
SELECT * FROM received_ticket;

DROP INDEX received_ticket_index;
CREATE INDEX received_ticket_index ON received_ticket_tmp (received_at);

 -- received tickets that are in the process of verifying
CREATE TABLE ticket_data_tmp (
   ticket_id           INTEGER NOT NULL PRIMARY KEY REFERENCES received_ticket_tmp(id),

   -- serial_number, alongside the entire row, will get purged after redemption is complete
   serial_number       BLOB NOT NULL UNIQUE,

   --    data will get purged after 80% of signers verifies it
   data                BLOB
);

INSERT INTO ticket_data_tmp
SELECT * FROM ticket_data;

DROP TABLE ticket_data;
ALTER TABLE ticket_data_tmp RENAME TO ticket_data;

-- result of a verification from a single signer (API)
CREATE TABLE ticket_verification_tmp (
   ticket_id           INTEGER NOT NULL REFERENCES received_ticket_tmp(id),
   signer_id           INTEGER NOT NULL,
   verified_at         TIMESTAMP WITHOUT TIME ZONE NOT NULL,
   accepted            BOOLEAN NOT NULL,
   
   PRIMARY KEY (ticket_id, signer_id)
);

DROP INDEX ticket_verification_index;
CREATE INDEX ticket_verification_index ON ticket_verification_tmp (ticket_id);

DROP TABLE ticket_verification;
ALTER TABLE ticket_verification_tmp RENAME TO ticket_verification;

-- verified tickets that are yet to be redeemed
CREATE TABLE verified_tickets_tmp (
   ticket_id           INTEGER NOT NULL PRIMARY KEY REFERENCES received_ticket_tmp(id),
   proposal_id         INTEGER REFERENCES redemption_proposals(proposal_id)
);

DROP INDEX verified_tickets_index;
CREATE INDEX verified_tickets_index ON verified_tickets_tmp (proposal_id);

DROP TABLE verified_tickets;
ALTER TABLE verified_tickets_tmp RENAME TO verified_tickets;

DROP TABLE received_ticket;
ALTER TABLE received_ticket_tmp RENAME TO received_ticket;

DROP TABLE shared_keys;
ALTER TABLE shared_keys_tmp RENAME TO shared_keys;
