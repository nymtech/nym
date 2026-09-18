/*
 * Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */

-- An `issued_ticketbook` row is the only thing that stops a deposit being spent on a second
-- ticketbook, but it is pruned a couple of days after the book it describes expires. The deposit
-- outlives it indefinitely: the ecash contract records no notion of a deposit having been
-- consumed, so a fresh withdrawal request against a pruned deposit is indistinguishable from a
-- first one and the same payment mints another book.
--
-- This table records that a deposit has been paid out and is never pruned, so retention of the
-- issuance data itself stays free to be as short as the merkle challenge routes need.
CREATE TABLE used_deposit
(
    deposit_id INTEGER NOT NULL PRIMARY KEY
);

-- every deposit we still hold an issuance record for has, by definition, been used.
-- deposits whose records were already pruned before this migration cannot be recovered; each of
-- those can be spent once more, and that issuance is what finally records them here.
INSERT INTO used_deposit (deposit_id)
SELECT deposit_id
FROM issued_ticketbook;
