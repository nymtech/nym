/*
 * Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */

CREATE TABLE metadata (
    id INTEGER PRIMARY KEY CHECK (id = 0),
    latest_network_id INTEGER REFERENCES network(id),
    
    master_mnemonic TEXT NOT NULL,
    rpc_endpoint TEXT NOT NULL
);

CREATE TABLE network (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    created_at TIMESTAMP WITHOUT TIME ZONE NOT NULL,

    mixnet_contract_id INTEGER NOT NULL REFERENCES contract(id),
    vesting_contract_id INTEGER NOT NULL REFERENCES contract(id),
    ecash_contract_id INTEGER NOT NULL REFERENCES contract(id),
    cw3_multisig_contract_id INTEGER NOT NULL REFERENCES contract(id),
    cw4_group_contract_id INTEGER NOT NULL REFERENCES contract(id),
    dkg_contract_id INTEGER NOT NULL REFERENCES contract(id),

    rewarder_address TEXT NOT NULL REFERENCES account(address),
    ecash_holding_account_address TEXT NOT NULL REFERENCES account(address)
);

CREATE TABLE contract (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    address TEXT NOT NULL,
    admin_address TEXT NOT NULL REFERENCES account(address)
);

CREATE TABLE account (
    address TEXT NOT NULL UNIQUE,
    mnemonic TEXT NOT NULL
);