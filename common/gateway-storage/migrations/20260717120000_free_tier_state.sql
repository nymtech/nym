/*
 * Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */

-- Per-public-key free-tier state, keyed by the WireGuard peer public key.
--
-- DELIBERATELY NOT a foreign key to wireguard_peer: this record must OUTLIVE the
-- peer. It is the source of truth for the rolling claim guard and for resuming a
-- trial after an idle-reaped peer reconnects, so it cannot be tied to the peer's
-- lifecycle. A FK would break that either way:
--   * ON DELETE CASCADE would drop the record when the peer is removed, letting a
--     Sybil reset the claim guard (and burning a legit user's remaining trial)
--     just by forcing a peer removal + re-add.
--   * ON DELETE RESTRICT/NO ACTION would block removing/reaping a free peer while
--     its record exists, breaking the peer lifecycle.
-- Stale rows (granted_at older than the claim window) carry no guard/resume value
-- and can be GC'd periodically; a re-claim replaces the row in place.
-- `granted_at` is the single timestamp driving both the time cap and the guard.
CREATE TABLE free_tier_state
(
    public_key TEXT      NOT NULL PRIMARY KEY,
    granted_at TIMESTAMP NOT NULL,
    is_free    BOOLEAN   NOT NULL DEFAULT 1
);
