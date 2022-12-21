/*
 * Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

ALTER TABLE rewarding_report RENAME TO _rewarding_report_old;

CREATE TABLE rewarding_report
(
    epoch_rewarding_id           INTEGER NOT NULL,

    eligible_mixnodes            INTEGER NOT NULL,

    possibly_unrewarded_mixnodes INTEGER NOT NULL,

    FOREIGN KEY (epoch_rewarding_id) REFERENCES epoch_rewarding (id)
);

INSERT INTO rewarding_report (epoch_rewarding_id, eligible_mixnodes, possibly_unrewarded_mixnodes)
SELECT epoch_rewarding_id, eligible_mixnodes, possibly_unrewarded_mixnodes
FROM _rewarding_report_old;

DROP TABLE _rewarding_report_old;
DROP TABLE failed_gateway_reward_chunk;
DROP TABLE possibly_unrewarded_gateway;