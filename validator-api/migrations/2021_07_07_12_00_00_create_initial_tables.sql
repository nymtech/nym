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

CREATE TABLE current_daily_mixnode_report
(
    node_id          INTEGER NOT NULL,

    most_recent_ipv4 INTEGER NOT NULL,
    most_recent_ipv6 INTEGER NOT NULL,

    last_hour_ipv4   INTEGER NOT NULL,
    last_hour_ipv6   INTEGER NOT NULL,

    last_24h_ipv4    INTEGER NOT NULL,
    last_24h_ipv6    INTEGER NOT NULL,

    FOREIGN KEY (node_id) REFERENCES mixnode_details (id)
);

-- this will hold information regarding uptime on nth day after mixnode was created
create TABLE mixnode_historical_uptime
(
    id          INTEGER NOT NULL,
    -- 'YYYY-MM-DD'
    date        VARCHAR NOT NULL,
    -- 24h uptimes for that day
    ipv4_uptime INTEGER,
    ipv6_uptime INTEGER,

    FOREIGN KEY (id) REFERENCES mixnode_details (id)
);

create table mixnode_ipv4_status
(
    node_id   INTEGER NOT NULL,
    up        BOOLEAN NOT NULL,
    timestamp INTEGER,

    FOREIGN KEY (node_id) REFERENCES mixnode_details (id)
);

create table mixnode_ipv6_status
(
    node_id   INTEGER NOT NULL,
    up        BOOLEAN NOT NULL,
    timestamp INTEGER,

    FOREIGN KEY (node_id) REFERENCES mixnode_details (id)
);


CREATE TABLE current_daily_gateway_report
(
    node_id          INTEGER NOT NULL,

    most_recent_ipv4 INTEGER NOT NULL,
    most_recent_ipv6 INTEGER NOT NULL,

    last_hour_ipv4   INTEGER NOT NULL,
    last_hour_ipv6   INTEGER NOT NULL,

    last_24h_ipv4    INTEGER NOT NULL,
    last_24h_ipv6    INTEGER NOT NULL,

    FOREIGN KEY (node_id) REFERENCES gateway_details (id)
);

-- this will hold information regarding uptime on nth day after gateway was created
create TABLE gateway_historical_uptime
(
    node_id     INTEGER NOT NULL,
    -- 'YYYY-MM-DD'
    date        VARCHAR NOT NULL,
    -- 24h uptimes for that day
    ipv4_uptime INTEGER,
    ipv6_uptime INTEGER,

    FOREIGN KEY (node_id) REFERENCES gateway_details (id)
);

create table gateway_ipv4_status
(
    node_id   INTEGER NOT NULL,
    up        BOOLEAN NOT NULL,
    timestamp INTEGER,

    FOREIGN KEY (node_id) REFERENCES gateway_details (id)
);

create table gateway_ipv6_status
(
    node_id   INTEGER NOT NULL,
    up        BOOLEAN NOT NULL,
    timestamp INTEGER,

    FOREIGN KEY (node_id) REFERENCES gateway_details (id)
);

-- indices for faster lookups
CREATE INDEX `mixnode_ipv4_status_index` ON `mixnode_ipv4_status` (`node_id`, `timestamp` desc);
CREATE INDEX `mixnode_ipv6_status_index` ON `mixnode_ipv6_status` (`node_id`, `timestamp` desc);

CREATE INDEX `gateway_ipv4_status_index` ON `gateway_ipv4_status` (`node_id`, `timestamp` desc);
CREATE INDEX `gateway_ipv6_status_index` ON `gateway_ipv6_status` (`node_id`, `timestamp` desc);
