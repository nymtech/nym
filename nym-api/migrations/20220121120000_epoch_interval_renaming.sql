/*
 * Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

ALTER TABLE epoch_rewarding
    RENAME TO interval_rewarding;
ALTER TABLE interval_rewarding
    RENAME COLUMN epoch_timestamp TO interval_start_timestamp;

-- default exists here since otherwise a column couldn't have been added
ALTER TABLE interval_rewarding
    ADD COLUMN interval_end_timestamp INTEGER NOT NULL default -1;


ALTER TABLE rewarding_report
    RENAME TO _rewarding_report_old;
CREATE TABLE rewarding_report
(
    interval_rewarding_id        INTEGER NOT NULL,

    eligible_mixnodes            INTEGER NOT NULL,

    possibly_unrewarded_mixnodes INTEGER NOT NULL,

    FOREIGN KEY (interval_rewarding_id) REFERENCES interval_rewarding (id)
);

INSERT INTO rewarding_report (interval_rewarding_id, eligible_mixnodes, possibly_unrewarded_mixnodes)
SELECT epoch_rewarding_id, eligible_mixnodes, possibly_unrewarded_mixnodes
FROM _rewarding_report_old;


-- I'm not 100% sure whether this is actually required, but better safe than sorry
ALTER TABLE failed_mixnode_reward_chunk
    RENAME TO _failed_mixnode_reward_chunk_old;
CREATE TABLE failed_mixnode_reward_chunk
(
    id                INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    error_message     VARCHAR NOT NULL,

    reward_summary_id INTEGER NOT NULL,

    FOREIGN KEY (reward_summary_id) REFERENCES interval_rewarding (id)
);

INSERT INTO failed_mixnode_reward_chunk (id, error_message, reward_summary_id)
SELECT id, error_message, reward_summary_id
FROM _failed_mixnode_reward_chunk_old;


-- yay for SQLite not having option to change foreign key in alter table

ALTER TABLE possibly_unrewarded_mixnode RENAME TO _possibly_unrewarded_mixnode_old;

CREATE TABLE possibly_unrewarded_mixnode
(
    id                             INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    identity                       VARCHAR NOT NULL,
    uptime                         INTEGER NOT NULL,

    failed_mixnode_reward_chunk_id INTEGER NOT NULL,

    FOREIGN KEY (failed_mixnode_reward_chunk_id) REFERENCES failed_mixnode_reward_chunk (id)
);

INSERT INTO possibly_unrewarded_mixnode (id, identity, uptime, failed_mixnode_reward_chunk_id)
SELECT id, identity, uptime, failed_mixnode_reward_chunk_id
FROM _possibly_unrewarded_mixnode_old;

DROP TABLE _rewarding_report_old;
DROP TABLE _failed_mixnode_reward_chunk_old;
DROP TABLE _possibly_unrewarded_mixnode_old;