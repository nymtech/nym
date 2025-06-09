/*
 * Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */


CREATE TABLE network_old
(
    id                            INTEGER                     NOT NULL PRIMARY KEY AUTOINCREMENT,
    name                          TEXT                        NOT NULL,
    created_at                    TIMESTAMP WITHOUT TIME ZONE NOT NULL,

    mixnet_contract_id            INTEGER                     NOT NULL REFERENCES contract (id),
    vesting_contract_id           INTEGER                     NOT NULL REFERENCES contract (id),
    ecash_contract_id             INTEGER                     NOT NULL REFERENCES contract (id),
    cw3_multisig_contract_id      INTEGER                     NOT NULL REFERENCES contract (id),
    cw4_group_contract_id         INTEGER                     NOT NULL REFERENCES contract (id),
    dkg_contract_id               INTEGER                     NOT NULL REFERENCES contract (id),

    rewarder_address              TEXT                        NOT NULL REFERENCES account (address),
    ecash_holding_account_address TEXT                        NOT NULL REFERENCES account (address)
);

INSERT INTO network_old
SELECT *
from network;

DROP TABLE network;

CREATE TABLE network
(
    id                            INTEGER                     NOT NULL PRIMARY KEY AUTOINCREMENT,
    name                          TEXT                        NOT NULL,
    created_at                    TIMESTAMP WITHOUT TIME ZONE NOT NULL,

    mixnet_contract_id            INTEGER                     NOT NULL REFERENCES contract (id),
    vesting_contract_id           INTEGER                     NOT NULL REFERENCES contract (id),
    ecash_contract_id             INTEGER                     NOT NULL REFERENCES contract (id),
    cw3_multisig_contract_id      INTEGER                     NOT NULL REFERENCES contract (id),
    cw4_group_contract_id         INTEGER                     NOT NULL REFERENCES contract (id),
    dkg_contract_id               INTEGER                     NOT NULL REFERENCES contract (id),
    performance_contract_id       INTEGER                     NOT NULL REFERENCES contract (id),

    rewarder_address              TEXT                        NOT NULL REFERENCES account (address),
    ecash_holding_account_address TEXT                        NOT NULL REFERENCES account (address)
);

CREATE TABLE authorised_network_monitor
(
    id         INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    network_id INTEGER NOT NULL REFERENCES network (id),
    address    TEXT    NOT NULL REFERENCES account (address)
);
