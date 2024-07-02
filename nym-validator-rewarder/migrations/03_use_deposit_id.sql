/*
 * Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */

DROP TABLE validated_deposit;
CREATE TABLE validated_deposit
(
    operator_identity_bs58 TEXT    NOT NULL,
    credential_id          INTEGER NOT NULL,
    deposit_id             INTEGER NOT NULL,

    signed_plaintext       BLOB    NOT NULL,
    signature_bs58         TEXT    NOT NULL
);

-- evidence of attempting to re-use the same deposit id for multiple credentials
DROP TABLE double_signing_evidence;
CREATE TABLE double_signing_evidence
(
    operator_identity_bs58 TEXT    NOT NULL,
    credential_id          INTEGER NOT NULL,
    original_credential_id INTEGER NOT NULL,
    deposit_id             INTEGER NOT NULL,

    signed_plaintext       BLOB    NOT NULL,
    signature_bs58         TEXT    NOT NULL
);