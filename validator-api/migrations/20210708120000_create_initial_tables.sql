CREATE TABLE mixnode_details
(
    id      INTEGER PRIMARY KEY AUTOINCREMENT,
    owner   VARCHAR NOT NULL,
    pub_key VARCHAR NOT NULL UNIQUE
);

CREATE TABLE gateway_details
(
    id      INTEGER PRIMARY KEY AUTOINCREMENT,
    owner   VARCHAR NOT NULL,
    pub_key VARCHAR NOT NULL UNIQUE
);

create TABLE mixnode_historical_uptime
(
    mixnode_details_id INTEGER NOT NULL,

    -- 'YYYY-MM-DD'
    date               VARCHAR NOT NULL,

    -- 24h uptimes for that day
    ipv4_uptime        INTEGER,
    ipv6_uptime        INTEGER,

    FOREIGN KEY (mixnode_details_id) REFERENCES mixnode_details (id)
);

create table mixnode_ipv4_status
(
    mixnode_details_id INTEGER NOT NULL,
    up                 BOOLEAN NOT NULL,
    timestamp          INTEGER NOT NULL,

    FOREIGN KEY (mixnode_details_id) REFERENCES mixnode_details (id)
);

create table mixnode_ipv6_status
(
    mixnode_details_id INTEGER NOT NULL,
    up                 BOOLEAN NOT NULL,
    timestamp          INTEGER NOT NULL,

    FOREIGN KEY (mixnode_details_id) REFERENCES mixnode_details (id)
);

create TABLE gateway_historical_uptime
(
    gateway_details_id INTEGER NOT NULL,

    -- 'YYYY-MM-DD'
    date                VARCHAR NOT NULL,

    -- 24h uptimes for that day
    ipv4_uptime         INTEGER,
    ipv6_uptime         INTEGER,

    FOREIGN KEY (gateway_details_id) REFERENCES gateway_details (id)
);

create table gateway_ipv4_status
(
    gateway_details_id INTEGER NOT NULL,
    up                  BOOLEAN NOT NULL,
    timestamp           INTEGER NOT NULL,

    FOREIGN KEY (gateway_details_id) REFERENCES gateway_details (id)
);

create table gateway_ipv6_status
(
    gateway_details_id INTEGER NOT NULL,
    up                  BOOLEAN NOT NULL,
    timestamp           INTEGER NOT NULL,

    FOREIGN KEY (gateway_details_id) REFERENCES gateway_details (id)
);

-- indices for faster lookups
CREATE
INDEX `mixnode_ipv4_status_index` ON `mixnode_ipv4_status` (`mixnode_details_id`, `timestamp` desc);
CREATE
INDEX `mixnode_ipv6_status_index` ON `mixnode_ipv6_status` (`mixnode_details_id`, `timestamp` desc);

CREATE
INDEX `gateway_ipv4_status_index` ON `gateway_ipv4_status` (`gateway_details_id`, `timestamp` desc);
CREATE
INDEX `gateway_ipv6_status_index` ON `gateway_ipv6_status` (`gateway_details_id`, `timestamp` desc);
