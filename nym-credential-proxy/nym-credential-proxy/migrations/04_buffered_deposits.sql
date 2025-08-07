/*
 * Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */

CREATE TABLE ecash_deposit
(
    -- id assigned [by the contract] to the deposit
    deposit_id                  INTEGER PRIMARY KEY         NOT NULL,

    -- associated tx hash
    deposit_tx_hash             TEXT                        NOT NULL,

    -- indication of when the deposit request has been created
    -- (so that based on block timestamp we could potentially determine latency)
    requested_on                TIMESTAMP WITHOUT TIME ZONE NOT NULL,

    -- the amount put in the deposit (informative, as we expect this to change in the future)
    deposit_amount              TEXT                        NOT NULL,

    -- the private key generated for the purposes of the deposit (the public component has been put in the transaction)
    ed25519_deposit_private_key BLOB                        NOT NULL
);


INSERT INTO ecash_deposit(deposit_id, deposit_tx_hash, requested_on, deposit_amount, ed25519_deposit_private_key)
SELECT deposit_id, deposit_tx_hash, requested_on, deposit_amount, ed25519_deposit_private_key
FROM ticketbook_deposit;


CREATE TABLE ecash_deposit_usage
(
    deposit_id               INTEGER PRIMARY KEY REFERENCES ecash_deposit (deposit_id),
    ticketbooks_requested_on TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    client_pubkey            BLOB                        NOT NULL,
    request_uuid             TEXT UNIQUE                 NOT NULL,

    -- this has to be improved later on to resume issuance or something, but for now it's fine
    ticketbook_request_error TEXT
);

INSERT INTO ecash_deposit_usage(deposit_id, ticketbooks_requested_on, client_pubkey, request_uuid)
SELECT deposit_id, 0, client_pubkey, request_uuid
FROM ticketbook_deposit;


CREATE TABLE partial_blinded_wallet_new
(
    corresponding_deposit INTEGER                     NOT NULL REFERENCES ecash_deposit_usage (deposit_id),
    epoch_id              INTEGER                     NOT NULL,
    expiration_date       DATE                        NOT NULL,
    node_id               INTEGER                     NOT NULL,
    created               TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    blinded_signature     BLOB                        NOT NULL
);

CREATE TABLE partial_blinded_wallet_failure_new
(
    corresponding_deposit INTEGER                     NOT NULL REFERENCES ecash_deposit_usage (deposit_id),
    epoch_id              INTEGER                     NOT NULL,
    expiration_date       DATE                        NOT NULL,
    node_id               INTEGER                     NOT NULL,
    created               TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    failure_message       TEXT                        NOT NULL
);

INSERT INTO partial_blinded_wallet_new
SELECT *
from partial_blinded_wallet;
INSERT INTO partial_blinded_wallet_failure_new
SELECT *
from partial_blinded_wallet_failure;

DROP TABLE partial_blinded_wallet;
DROP TABLE partial_blinded_wallet_failure;

