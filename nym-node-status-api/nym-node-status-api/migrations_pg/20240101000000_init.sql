CREATE TABLE gateways
(
  id                      SERIAL PRIMARY KEY,
  gateway_identity_key    VARCHAR NOT NULL UNIQUE,
  self_described          VARCHAR NOT NULL,
  explorer_pretty_bond    VARCHAR,
  last_probe_result       VARCHAR,
  last_probe_log          VARCHAR,
  config_score            INTEGER NOT NULL DEFAULT 0,
  config_score_successes  DOUBLE PRECISION NOT NULL DEFAULT 0,
  config_score_samples    DOUBLE PRECISION NOT NULL DEFAULT 0,
  routing_score           INTEGER NOT NULL DEFAULT 0,
  routing_score_successes DOUBLE PRECISION NOT NULL DEFAULT 0,
  routing_score_samples   DOUBLE PRECISION NOT NULL DEFAULT 0,
  test_run_samples        DOUBLE PRECISION NOT NULL DEFAULT 0,
  last_testrun_utc        BIGINT,
  last_updated_utc        BIGINT NOT NULL,
  bonded                  BOOLEAN NOT NULL DEFAULT FALSE,
  blacklisted             BOOLEAN NOT NULL DEFAULT FALSE,
  performance             INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_gateway_description_gateway_identity_key ON gateways (gateway_identity_key);


CREATE TABLE mixnodes (
    id               SERIAL PRIMARY KEY,
    identity_key     VARCHAR NOT NULL UNIQUE,
    mix_id           BIGINT NOT NULL UNIQUE,
    bonded           BOOLEAN NOT NULL DEFAULT FALSE,
    total_stake      BIGINT NOT NULL,
    host             VARCHAR NOT NULL,
    http_api_port    BIGINT NOT NULL,
    blacklisted      BOOLEAN NOT NULL DEFAULT FALSE,
    full_details     VARCHAR,
    self_described   VARCHAR,
    last_updated_utc BIGINT NOT NULL,
    is_dp_delegatee  BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE INDEX idx_mixnodes_mix_id ON mixnodes (mix_id);
CREATE INDEX idx_mixnodes_identity_key ON mixnodes (identity_key);

CREATE TABLE mixnode_description (
    id               SERIAL PRIMARY KEY,
    mix_id           BIGINT UNIQUE NOT NULL,
    moniker          VARCHAR,
    website          VARCHAR,
    security_contact VARCHAR,
    details          VARCHAR,
    last_updated_utc BIGINT NOT NULL,
    FOREIGN KEY (mix_id) REFERENCES mixnodes (mix_id)
);

-- Indexes for description table
CREATE INDEX idx_mixnode_description_mix_id ON mixnode_description (mix_id);


CREATE TABLE summary
(
  key              VARCHAR PRIMARY KEY,
  value_json       VARCHAR,
  last_updated_utc BIGINT NOT NULL
);


CREATE TABLE summary_history
(
  id            SERIAL PRIMARY KEY,
  date          VARCHAR UNIQUE NOT NULL,
  timestamp_utc BIGINT NOT NULL,
  value_json    VARCHAR
);

CREATE INDEX idx_summary_history_timestamp_utc ON summary_history (timestamp_utc);
CREATE INDEX idx_summary_history_date ON summary_history (date);


CREATE TABLE gateway_description (
    id                   SERIAL PRIMARY KEY,
    gateway_identity_key VARCHAR UNIQUE NOT NULL,
    moniker              VARCHAR,
    website              VARCHAR,
    security_contact     VARCHAR,
    details              VARCHAR,
    last_updated_utc     BIGINT NOT NULL,
    FOREIGN KEY (gateway_identity_key) REFERENCES gateways (gateway_identity_key)
);


CREATE TABLE mixnode_daily_stats (
    id               SERIAL PRIMARY KEY,
    mix_id           BIGINT NOT NULL,
    total_stake      BIGINT NOT NULL,
    date_utc         VARCHAR NOT NULL,
    packets_received INTEGER DEFAULT 0,
    packets_sent     INTEGER DEFAULT 0,
    packets_dropped  INTEGER DEFAULT 0,
    FOREIGN KEY (mix_id) REFERENCES mixnodes (mix_id),
    UNIQUE (mix_id, date_utc) -- This constraint automatically creates an index
);


CREATE TABLE testruns
(
  id            SERIAL PRIMARY KEY,
  gateway_id    INTEGER NOT NULL,
  status        INTEGER NOT NULL, -- 0=pending, 1=in-progress, 2=complete
  timestamp_utc BIGINT NOT NULL,
  ip_address    VARCHAR NOT NULL,
  log           VARCHAR NOT NULL,
  FOREIGN KEY (gateway_id) REFERENCES gateways (id)
);