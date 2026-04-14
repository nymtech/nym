-- Persistent port-scan results (e.g. exit policy) live beside last_probe_result in the API,
-- not embedded in the probe JSON blob, so frequent probe runs do not wipe 14-day port checks.
ALTER TABLE gateways
    ADD COLUMN IF NOT EXISTS ports_check JSONB;

ALTER TABLE gateways
    ADD COLUMN IF NOT EXISTS last_ports_check_utc BIGINT;
