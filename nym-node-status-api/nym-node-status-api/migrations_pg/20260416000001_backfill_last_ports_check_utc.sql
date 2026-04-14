-- `20260416000000` may have copied `ports_check` from legacy JSON without setting `last_ports_check_utc`.
UPDATE gateways
SET last_ports_check_utc = last_updated_utc
WHERE ports_check IS NOT NULL
  AND jsonb_typeof(ports_check) = 'object'
  AND last_ports_check_utc IS NULL;
