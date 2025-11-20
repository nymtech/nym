/*
 * Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */


CREATE TABLE account
(
    address  TEXT NOT NULL PRIMARY KEY UNIQUE,
    mnemonic TEXT NOT NULL
);

CREATE TABLE nyxd
(
    id             INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    rpc_endpoint   TEXT    NOT NULL,
    master_address TEXT    NOT NULL REFERENCES account (address)
);

CREATE table nym_api
(
    network_id INTEGER NOT NULL PRIMARY KEY REFERENCES localnet_metadata (id),
    endpoint   TEXT    NOT NULL
);

CREATE TABLE contract
(
    -- note: I'm purposely not using contract address as primary key,
    -- as you can have the same addresses for different contracts (on different instances of localnets)
    -- as addressing is semi-kinda deterministic-ish
    id            INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    name          TEXT    NOT NULL,
    address       TEXT    NOT NULL,
    admin_address TEXT    NOT NULL REFERENCES account (address)
);

CREATE TABLE localnet_metadata
(
    id         INTEGER                     NOT NULL PRIMARY KEY AUTOINCREMENT,
    -- human-readable name associated with the localnet (to have some unique prefix for containers)
    name       TEXT                        NOT NULL UNIQUE,
    created_at TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE localnet_contracts
(
    metadata_id              INTEGER NOT NULL PRIMARY KEY REFERENCES localnet_metadata (id),

    mixnet_contract_id       INTEGER NOT NULL REFERENCES contract (id),
    vesting_contract_id      INTEGER NOT NULL REFERENCES contract (id),
    ecash_contract_id        INTEGER NOT NULL REFERENCES contract (id),
    cw3_multisig_contract_id INTEGER NOT NULL REFERENCES contract (id),
    cw4_group_contract_id    INTEGER NOT NULL REFERENCES contract (id),
    dkg_contract_id          INTEGER NOT NULL REFERENCES contract (id),
    performance_contract_id  INTEGER NOT NULL REFERENCES contract (id)
);

CREATE TABLE localnet_auxiliary_accounts
(
    network_id                    INTEGER NOT NULL PRIMARY KEY REFERENCES localnet_metadata (id),

    rewarder_address              TEXT    NOT NULL REFERENCES account (address),
    ecash_holding_account_address TEXT    NOT NULL REFERENCES account (address)
);

CREATE TABLE localnet
(
    metadata_id INTEGER NOT NULL PRIMARY KEY REFERENCES localnet_metadata (id),
    nyxd_id     INTEGER NOT NULL REFERENCES nyxd (id)
);


-- keep it separate to more easily support testing having multiple network monitors
CREATE TABLE authorised_network_monitor
(
    address    TEXT    NOT NULL PRIMARY KEY REFERENCES account (address),
    network_id INTEGER NOT NULL REFERENCES localnet_metadata (id)
);

CREATE TABLE metadata
(
    id                INTEGER PRIMARY KEY CHECK (id = 0),
    latest_network_id INTEGER REFERENCES localnet_metadata (id),
    latest_nyxd_id    INTEGER REFERENCES nyxd (id)
);

CREATE TABLE nym_node
(
    node_id              INTEGER NOT NULL,
    identity_key         TEXT    NOT NULL PRIMARY KEY,
    private_identity_key TEXT    NOT NULL,
    network_id           INTEGER NOT NULL REFERENCES localnet_metadata (id),
    owner_address        TEXT    NOT NULL REFERENCES account (address),
    gateway              BOOL    NOT NULL
);

INSERT OR IGNORE INTO metadata(id)
VALUES (0);
