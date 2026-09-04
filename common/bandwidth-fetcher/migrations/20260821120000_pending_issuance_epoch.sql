/*
 * Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */

-- The epoch a pending issuance is being collected under, settled once before the deposit is made
-- and reused on every attempt, including after a restart. Shares from two epochs cannot be
-- aggregated, so resolving it afresh on resume risks completing a collection under an epoch the
-- shares already gathered do not belong to.
--
-- NULL for rows written before this column existed: those resume by resolving the epoch again,
-- which is what they were doing all along.
ALTER TABLE pending_issuance
    ADD COLUMN dkg_epoch_id INTEGER;
