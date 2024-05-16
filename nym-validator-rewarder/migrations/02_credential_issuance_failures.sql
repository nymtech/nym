/*
 * Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */

CREATE TABLE validated_deposit
(
    operator_identity_bs58 TEXT    NOT NULL,
    credential_id          INTEGER NOT NULL,
    deposit_tx             TEXT    NOT NULL,

    signed_plaintext       BLOB    NOT NULL,
    signature_bs58         TEXT    NOT NULL
);

-- evidence of attempting to re-use the same deposit tx for multiple credentials
CREATE TABLE double_signing_evidence
(
    operator_identity_bs58 TEXT    NOT NULL,
    credential_id          INTEGER NOT NULL,
    original_credential_id INTEGER NOT NULL,
    deposit_tx             TEXT    NOT NULL,

    signed_plaintext       BLOB    NOT NULL,
    signature_bs58         TEXT    NOT NULL
);

-- evidence of foul play
CREATE TABLE issuance_evidence
(
    operator_account       TEXT    NOT NULL,
    operator_identity_bs58 TEXT    NOT NULL,

    credential_id          INTEGER NOT NULL,
    signed_plaintext       BLOB    NOT NULL,
    signature_bs58         TEXT    NOT NULL,

    failure_message        TEXT    NOT NULL
);

-- does not necessarily imply foul play, but something has gone wrong
CREATE TABLE issuance_validation_failure
(
    id                     INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,

    operator_account       TEXT    NOT NULL,
    operator_identity_bs58 TEXT    NOT NULL,

    failure_message        TEXT    NOT NULL
)