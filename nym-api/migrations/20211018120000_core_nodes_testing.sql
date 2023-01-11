/*
 * Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */


ALTER TABLE mixnode_historical_uptime
    RENAME TO _mixnode_historical_uptime_old;


CREATE TABLE mixnode_historical_uptime
(
    mixnode_details_id INTEGER NOT NULL,
    date               VARCHAR NOT NULL,
    uptime             INTEGER NOT NULL,
    FOREIGN KEY (mixnode_details_id)
        REFERENCES mixnode_details (id)
);

INSERT INTO mixnode_historical_uptime (mixnode_details_id, date, uptime)
SELECT mixnode_details_id, date, (ipv4_uptime + ipv6_uptime) / 2 as uptime
from _mixnode_historical_uptime_old;

DROP TABLE _mixnode_historical_uptime_old;

ALTER TABLE gateway_historical_uptime
    RENAME TO _gateway_historical_uptime_old;


CREATE TABLE gateway_historical_uptime
(
    gateway_details_id INTEGER NOT NULL,
    date               VARCHAR NOT NULL,
    uptime             INTEGER NOT NULL,
    FOREIGN KEY (gateway_details_id)
        REFERENCES gateway_details (id)
);

INSERT INTO gateway_historical_uptime (gateway_details_id, date, uptime)
SELECT gateway_details_id, date, (ipv4_uptime + ipv6_uptime) / 2 as uptime
from _gateway_historical_uptime_old;

DROP TABLE _gateway_historical_uptime_old;

CREATE TABLE mixnode_status
(
    mixnode_details_id INTEGER NOT NULL,
    reliability        INTEGER NOT NULL,
    timestamp          INTEGER NOT NULL,
    FOREIGN KEY (mixnode_details_id) REFERENCES mixnode_details (id)
);

INSERT INTO mixnode_status (mixnode_details_id, timestamp, reliability)
SELECT mixnode_ipv4_status.mixnode_details_id,
       mixnode_ipv4_status.timestamp,
       (mixnode_ipv4_status.up * 100 + mixnode_ipv6_status.up * 100) / 2 as reliability
FROM mixnode_ipv4_status
         JOIN mixnode_ipv6_status ON mixnode_ipv4_status.mixnode_details_id = mixnode_ipv6_status.mixnode_details_id AND
                                     mixnode_ipv4_status.timestamp = mixnode_ipv6_status.timestamp;

DROP TABLE mixnode_ipv4_status;
DROP TABLE mixnode_ipv6_status;

CREATE TABLE gateway_status
(
    gateway_details_id INTEGER NOT NULL,
    reliability        INTEGER NOT NULL,
    timestamp          INTEGER NOT NULL,
    FOREIGN KEY (gateway_details_id) REFERENCES gateway_details (id)
);

INSERT INTO gateway_status (gateway_details_id, timestamp, reliability)
SELECT gateway_ipv4_status.gateway_details_id,
       gateway_ipv4_status.timestamp,
       (gateway_ipv4_status.up * 100 + gateway_ipv6_status.up * 100) / 2 as reliability
FROM gateway_ipv4_status
         JOIN gateway_ipv6_status ON gateway_ipv4_status.gateway_details_id = gateway_ipv6_status.gateway_details_id AND
                                     gateway_ipv4_status.timestamp = gateway_ipv6_status.timestamp;

DROP TABLE gateway_ipv4_status;
DROP TABLE gateway_ipv6_status;

CREATE INDEX `mixnode_status_index` ON `mixnode_status` (`mixnode_details_id`, `timestamp` desc);
CREATE INDEX `gateway_status_index` ON `gateway_status` (`gateway_details_id`, `timestamp` desc);

CREATE TABLE testing_route (
    gateway_id INTEGER NOT NULL,
    layer1_mix_id INTEGER NOT NULL,
    layer2_mix_id INTEGER NOT NULL,
    layer3_mix_id INTEGER NOT NULL,
    monitor_run_id INTEGER NOT NULL,

    FOREIGN KEY (layer1_mix_id) REFERENCES mixnode_details (id),
    FOREIGN KEY (layer2_mix_id) REFERENCES mixnode_details (id),
    FOREIGN KEY (layer3_mix_id) REFERENCES mixnode_details (id),

    FOREIGN KEY (gateway_id) REFERENCES gateway_details (id),

    FOREIGN KEY (monitor_run_id) references monitor_run (id)
);
