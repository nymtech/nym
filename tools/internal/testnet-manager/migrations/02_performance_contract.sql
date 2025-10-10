/*
 * Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */

-- 1. Rename old table to preserve data
ALTER TABLE network
    RENAME TO network_old;

-- 2. Insert placeholder account (so that old networks would have _some_ value for performance contract)
INSERT INTO account (address, mnemonic)
VALUES ('n1tq2kggc6y44yqmnafh98vexxav8666cfkgvygf',
        'opinion scene salon slice noise easy security drift brown custom verb express old matrix mammal choose attract trash general staff manual elite destroy strategy');

-- 3. Insert placeholder contract and record its id
INSERT INTO contract (name, address, admin_address)
VALUES ('placeholder', 'n14gl07zh58rydd4k9tyw320zvqd79vrwnjj4x9g', 'n1tq2kggc6y44yqmnafh98vexxav8666cfkgvygf');

CREATE TEMP TABLE tmp_placeholder
(
    id INTEGER NOT NULL
);
INSERT INTO tmp_placeholder
VALUES (last_insert_rowid());


-- 4. Create the new network table with the new column
CREATE TABLE network
(
    id                            INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    name                          TEXT                              NOT NULL,
    created_at                    TIMESTAMP WITHOUT TIME ZONE       NOT NULL,

    mixnet_contract_id            INTEGER                           NOT NULL REFERENCES contract (id),
    vesting_contract_id           INTEGER                           NOT NULL REFERENCES contract (id),
    ecash_contract_id             INTEGER                           NOT NULL REFERENCES contract (id),
    cw3_multisig_contract_id      INTEGER                           NOT NULL REFERENCES contract (id),
    cw4_group_contract_id         INTEGER                           NOT NULL REFERENCES contract (id),
    dkg_contract_id               INTEGER                           NOT NULL REFERENCES contract (id),
    performance_contract_id       INTEGER                           NOT NULL REFERENCES contract (id),

    rewarder_address              TEXT                              NOT NULL REFERENCES account (address),
    ecash_holding_account_address TEXT                              NOT NULL REFERENCES account (address)
);

-- 5. Copy existing data into the new table
INSERT INTO network(id, name, created_at,
                    mixnet_contract_id, vesting_contract_id, ecash_contract_id,
                    cw3_multisig_contract_id, cw4_group_contract_id, dkg_contract_id,
                    performance_contract_id,
                    rewarder_address, ecash_holding_account_address)
SELECT n.id,
       n.name,
       n.created_at,
       n.mixnet_contract_id,
       n.vesting_contract_id,
       n.ecash_contract_id,
       n.cw3_multisig_contract_id,
       n.cw4_group_contract_id,
       n.dkg_contract_id,
       t.id, -- use the placeholder contract id
       n.rewarder_address,
       n.ecash_holding_account_address
FROM network_old AS n
         CROSS JOIN tmp_placeholder AS t;

-- 6. recreate metadata table due to change in FK
ALTER TABLE metadata
    RENAME TO metadata_old;

CREATE TABLE metadata
(
    id                INTEGER PRIMARY KEY CHECK (id = 0),
    latest_network_id INTEGER REFERENCES network (id),

    master_mnemonic   TEXT NOT NULL,
    rpc_endpoint      TEXT NOT NULL
);

INSERT INTO metadata
SELECT *
FROM metadata_old;

-- 7. recreate node table due to change in FK
ALTER Table node
    RENAME TO node_old;

CREATE TABLE node
(
    id            INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    identity_key  TEXT    NOT NULL,
    network_id    INTEGER NOT NULL REFERENCES network (id),

    -- i.e. mixnode or gateway
    bonded_type   TEXT    NOT NULL,
    owner_address TEXT    NOT NULL REFERENCES account (address)
);

INSERT INTO node
SELECT *
FROM node_old;

-- 8. Clean up
DROP TABLE tmp_placeholder;
DROP TABLE metadata_old;
DROP TABLE node_old;
DROP TABLE network_old;


CREATE TABLE authorised_network_monitor
(
    id         INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    network_id INTEGER NOT NULL REFERENCES network (id),
    address    TEXT    NOT NULL REFERENCES account (address)
);
