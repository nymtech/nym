CREATE TABLE gateways
(
  id                      INTEGER PRIMARY KEY AUTOINCREMENT,
  gateway_identity_key    VARCHAR NOT NULL UNIQUE,
  self_described          VARCHAR,
  explorer_pretty_bond    VARCHAR,
  last_probe_result       VARCHAR,
  last_probe_log          VARCHAR,
  config_score            INTEGER NOT NULL DEFAULT (0),
  config_score_successes  REAL    NOT NULL DEFAULT (0),
  config_score_samples    REAL    NOT NULL DEFAULT (0),
  routing_score           INTEGER NOT NULL DEFAULT (0),
  routing_score_successes REAL    NOT NULL DEFAULT (0),
  routing_score_samples   REAL    NOT NULL DEFAULT (0),
  test_run_samples        REAL    NOT NULL DEFAULT (0),
  last_testrun_utc        INTEGER,
  last_updated_utc        INTEGER NOT NULL,
  bonded INTEGER CHECK (bonded in (0, 1)) NOT NULL DEFAULT 0,
  blacklisted INTEGER CHECK (bonded in (0, 1)) NOT NULL DEFAULT 0,
  performance INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_gateway_description_gateway_identity_key ON gateways (gateway_identity_key);


CREATE TABLE mixnodes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    identity_key VARCHAR NOT NULL UNIQUE,
    mix_id INTEGER NOT NULL UNIQUE,
    bonded INTEGER CHECK (bonded in (0, 1)) NOT NULL DEFAULT 0,
    total_stake INTEGER NOT NULL,
    host VARCHAR NOT NULL,
    http_api_port INTEGER NOT NULL,
    blacklisted INTEGER CHECK (blacklisted in (0, 1)) NOT NULL DEFAULT 0,
    full_details VARCHAR,
    self_described VARCHAR,
    last_updated_utc INTEGER NOT NULL
  , is_dp_delegatee INTEGER CHECK (is_dp_delegatee IN (0, 1)) NOT NULL DEFAULT 0);
CREATE INDEX idx_mixnodes_mix_id ON mixnodes (mix_id);
CREATE INDEX idx_mixnodes_identity_key ON mixnodes (identity_key);


CREATE TABLE summary
(
  key              VARCHAR PRIMARY KEY,
  value_json       VARCHAR,
  last_updated_utc INTEGER NOT NULL
);


CREATE TABLE summary_history
(
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  date          VARCHAR UNIQUE NOT NULL,
  timestamp_utc INTEGER NOT NULL,
  value_json    VARCHAR
);
CREATE INDEX idx_summary_history_timestamp_utc ON summary_history (timestamp_utc);
CREATE INDEX idx_summary_history_date ON summary_history (date);
