-- Exit port check summary lives in `ports_check`, not inside `last_probe_result` JSON.
ALTER TABLE gateways ADD COLUMN IF NOT EXISTS ports_check JSONB;

UPDATE gateways
SET ports_check = (last_probe_result::jsonb -> 'ports_check')
WHERE last_probe_result IS NOT NULL
  AND btrim(last_probe_result) <> ''
  AND last_probe_result::jsonb ? 'ports_check';

UPDATE gateways
SET last_probe_result = (last_probe_result::jsonb - 'ports_check')::text
WHERE last_probe_result IS NOT NULL
  AND btrim(last_probe_result) <> ''
  AND last_probe_result::jsonb ? 'ports_check';
