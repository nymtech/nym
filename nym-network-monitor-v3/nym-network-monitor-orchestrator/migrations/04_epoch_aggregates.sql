/*
 * Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */

-- One row per ASSIGNMENT, which is what makes a node that measured badly distinguishable from a node
-- that was never measured. The performance contract's interface carries only a score, so a 0.0
-- meaning "the node was dead" and a 0.0 meaning "we never tested it" are indistinguishable once
-- published, and only the orchestrator can tell them apart.
--
-- It cannot do so from the tables that already exist. `node_test_state` keeps a single "last" value
-- per pairing rather than a log, and eviction DELETES expired `testrun_in_progress` rows outright,
-- so a lease that ran out leaves no trace. Over a window the orchestrator can therefore count
-- results and nothing else. A row written at assignment with no score is what makes the three states
-- derivable: a row carrying a score returned, a row still unscored was assigned and never returned,
-- and no row at all means never assigned. A probe that fails critically is deliberately never
-- submitted so that the lease expires and the node keeps its turn, which is exactly the second case,
-- and which is a MONITOR failure rather than a statement about the node.
--
-- Deliberately narrow and deliberately separate from `testrun`/`testrun_measurement`: aggregation
-- reads one value per assignment over a window, so a purpose-built table makes that read cheaper
-- than joining two wider ones per node per kind, and it carries its own retention schedule so that
-- the floor the aggregation window needs is a property of a table this concern owns rather than a
-- new constraint on `testrun_eviction_age`.

CREATE TABLE testrun_sample
(
    -- Surrogate primary key.
    id          INTEGER                                            NOT NULL PRIMARY KEY AUTOINCREMENT,

    -- The node the work was assigned against.
    node_id     INTEGER                                            NOT NULL REFERENCES nym_node (node_id),

    -- What the assignment was to measure. The ROLE is deliberately absent: an aggregate collapses
    -- every role and every tested address of a node into one value per kind, so recording the role
    -- here would store something nothing reads.
    test_kind   TEXT CHECK ( test_kind IN ('stress', 'liveness') ) NOT NULL,

    -- When the work was handed out. This is the timestamp an aggregation window selects on, and it
    -- is NOT the timestamp the resulting run is stored under: a result is stamped when it arrives,
    -- which is one probe later.
    assigned_at TIMESTAMP WITHOUT TIME ZONE                        NOT NULL,

    -- The score of the run that came back, normalised over the kind's expected measurement set.
    -- NULL while no result has arrived, which is the positive signal that what a window is missing
    -- is the monitor rather than the node.
    --
    -- No lease deadline is stored alongside it, because nothing would read one: the submission path
    -- already drops a result whose in-flight row has been reaped, so a sample can only ever be
    -- scored while its lease is live, and an aggregate is materialised once and never revisited, so
    -- an assignment still in flight when its window closes is one that contributed nothing.
    score       REAL CHECK ( score IS NULL OR (score >= 0.0 AND score <= 1.0) )
);

-- Supports the read aggregation makes: every sample of one (node, kind) within a window.
CREATE INDEX idx_testrun_sample_node_kind_assigned ON testrun_sample (node_id, test_kind, assigned_at);

-- ---------------------------------------------------------------------------
-- mixnet_epoch_aggregate: what a node was worth over one epoch, per kind
-- ---------------------------------------------------------------------------

-- Computed once, when an epoch begins, and served from here thereafter.
--
-- Materialised rather than computed on read because results keep arriving for runs whose assignment
-- already falls inside an anchored window, so the same query answered at two moments would give two
-- answers and a figure already handed to a consumer could move underneath it. A value destined for a
-- contract has to be stable.
--
-- Rebuildable rather than durable: discarding this table must not stop the orchestrator starting and
-- must never need a data migration to preserve. Whatever has been published lives in the contract,
-- and whatever has not can be recomputed while its window is still covered by retained samples.

CREATE TABLE mixnet_epoch_aggregate
(
    -- Absolute id of the mixnet epoch this value is filed under, as the mixnet contract counts them.
    -- Named in full rather than `epoch`, which this schema already uses for the unrelated sphinx
    -- `key_rotation_id`. The window it covers PRECEDES this epoch, which is what lets the value exist
    -- before the epoch ends.
    mixnet_epoch INTEGER                                            NOT NULL,

    node_id      INTEGER                                            NOT NULL REFERENCES nym_node (node_id),

    -- Which kind's runs were averaged. Held per kind rather than combined, because the weighting that
    -- turns per-kind values into a single performance figure is not decided here.
    test_kind    TEXT CHECK ( test_kind IN ('stress', 'liveness') ) NOT NULL,

    -- The mean of the scores of the runs that came back in the window.
    score        REAL CHECK ( score >= 0.0 AND score <= 1.0 )       NOT NULL,

    -- How many runs that mean was taken over. The only ground truth about how much evidence stands
    -- behind a value, since no relationship between a window and a kind's cadence can guarantee that
    -- any particular number of runs actually arrived - a node can be held by the other kind's lock,
    -- or simply not be reached by the sweep.
    samples      INTEGER CHECK ( samples > 0 )                      NOT NULL,

    -- A row exists only where something was measured, so an absent row is an absent value and can
    -- never be mistaken for a measured zero. Which KIND of nothing happened - assigned and silent, or
    -- never assigned at all - is answerable from `testrun_sample` while its retention holds, and is
    -- not recorded here: nothing reads it yet, and the policy that will is the submission change's.
    PRIMARY KEY (mixnet_epoch, node_id, test_kind)
);

-- ---------------------------------------------------------------------------
-- testrun_in_progress: carry the sample the assignment created
-- ---------------------------------------------------------------------------

-- A result reports only the node and the address it probed, so the row it completes has to be found
-- from what the orchestrator recorded at dispatch. The in-flight row is already that record - it is
-- where the kind and role a result gets filed under come from - so it carries the sample's id as
-- well, and an arriving score is written by primary key.
--
-- The link points from the lease to the sample rather than the other way around because of the order
-- the rows come into existence: the sample exists from the moment work is handed out, while the
-- `testrun` a result produces is only inserted when that result arrives.
--
-- Rebuilt rather than altered, since the column is NOT NULL and existing rows have no sample to
-- point at. Discarding them costs nothing, by the same argument the per-kind migration made: every
-- lease is orphaned by the restart that deploys this migration, so the rows would only keep their
-- nodes out of the assignment queue until the first eviction sweep.

DROP TABLE testrun_in_progress;

CREATE TABLE testrun_in_progress
(
    -- The node currently being tested.
    node_id     INTEGER PRIMARY KEY REFERENCES nym_node (node_id)    NOT NULL,

    -- When the in-progress run was dispatched.
    started_at  TIMESTAMP WITHOUT TIME ZONE                          NOT NULL,

    -- When the lease expires and the row becomes reapable, materialised as `started_at` plus the
    -- dispatching kind's lease budget. Stored rather than derived so the eviction sweep stays a
    -- single `expires_at < ?` comparison and never has to learn about kinds: a future kind that
    -- runs for minutes needs no change to eviction.
    expires_at  TIMESTAMP WITHOUT TIME ZONE                          NOT NULL,

    -- What the run was dispatched to measure.
    test_kind   TEXT CHECK ( test_kind IN ('stress', 'liveness') )   NOT NULL,

    -- Which role of the node the run was dispatched against. This is the AUTHORITATIVE source of
    -- the role when the result comes back: the completed run records the role it measured, while
    -- the submission reports only the node and the address, so without it the orchestrator would
    -- depend on the agent echoing back a value the orchestrator itself chose.
    tested_role TEXT CHECK ( tested_role IN ('mixnode', 'gateway') ) NOT NULL,

    -- The sample this assignment created, which an arriving result scores. Deliberately without a
    -- cascade: releasing the lease, whether by a result or by expiry, must leave the sample behind,
    -- because an assignment that never returned is exactly what an aggregate's coverage reads.
    sample_id   INTEGER                                              NOT NULL REFERENCES testrun_sample (id)
);
