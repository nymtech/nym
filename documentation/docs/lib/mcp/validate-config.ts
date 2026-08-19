// Config validator for the mix-tunnel / mix-fetch setup options
// (SetupMixTunnelOpts).
//
// The field set below is a SNAPSHOT of one SDK version's generated types. Two
// design choices follow from that:
//
//   - Unknown keys are WARNINGS, not errors. A newer SDK may add a field; if we
//     rejected unknowns, valid config would become a false error the moment the
//     SDK moved ahead of this snapshot. A warning still catches typos without
//     that failure mode.
//   - We do NOT encode mixnet performance thresholds (SURB counts, timeouts).
//     Those depend on the transport internals of a specific stack and cannot be
//     verified here; only the type checks and the privacy notes hold across
//     versions.

export type FieldType = 'string' | 'number' | 'boolean';

/** SetupMixTunnelOpts fields, from the generated TypeDoc. All optional. */
export const SETUP_MIX_TUNNEL_FIELDS: Record<string, FieldType> = {
  preferredIpr: 'string',
  clientId: 'string',
  forceTls: 'boolean',
  disablePoissonTraffic: 'boolean',
  disableCoverTraffic: 'boolean',
  openReplySurbs: 'number',
  dataReplySurbs: 'number',
  primaryDns: 'string',
  fallbackDns: 'string',
  storagePassphrase: 'string',
  connectTimeoutMs: 'number',
  dnsTimeoutMs: 'number',
  tcpKeepaliveMs: 'number',
  tcpBufferSize: 'number',
  maxRedirects: 'number',
  debug: 'boolean',
};

export interface ValidationResult {
  /** false only on hard errors (wrong shape or wrong field types). */
  valid: boolean;
  errors: string[];
  warnings: string[];
}

export function validateSetupMixTunnelOpts(config: unknown): ValidationResult {
  const errors: string[] = [];
  const warnings: string[] = [];

  if (config === null || typeof config !== 'object' || Array.isArray(config)) {
    return { valid: false, errors: ['Config must be an object (SetupMixTunnelOpts).'], warnings };
  }

  const cfg = config as Record<string, unknown>;
  const known = Object.keys(SETUP_MIX_TUNNEL_FIELDS);

  for (const [key, value] of Object.entries(cfg)) {
    const expected = SETUP_MIX_TUNNEL_FIELDS[key];
    if (!expected) {
      const near = closestKey(key, known);
      warnings.push(
        near
          ? `Unknown option "${key}". Did you mean "${near}"? The field list is a snapshot of one SDK version.`
          : `Unknown option "${key}". Not in this SDK snapshot; verify it against your installed version if intentional.`,
      );
      continue;
    }
    if (typeof value !== expected) {
      const got = Array.isArray(value) ? 'array' : value === null ? 'null' : typeof value;
      errors.push(`Option "${key}" should be ${expected}, got ${got}.`);
    }
  }

  // Privacy tradeoffs. Cover traffic and Poisson send timing are the anonymity
  // mechanism, not perf knobs, so disabling them is a real downgrade. This is
  // true regardless of SDK version.
  if (cfg.disableCoverTraffic === true) {
    warnings.push(
      'disableCoverTraffic: true removes cover traffic, shrinking the anonymity set an observer has to reason over. Only disable it if you understand the tradeoff.',
    );
  }
  if (cfg.disablePoissonTraffic === true) {
    warnings.push(
      'disablePoissonTraffic: true turns off Poisson send timing, weakening resistance to traffic-analysis. Only disable it if you understand the tradeoff.',
    );
  }

  return { valid: errors.length === 0, errors, warnings };
}

/** Nearest known key by edit distance, but only if it is a plausible typo. */
function closestKey(key: string, keys: string[]): string | null {
  const lk = key.toLowerCase();
  let best: string | null = null;
  let bestDist = Infinity;
  for (const k of keys) {
    const dist = levenshtein(lk, k.toLowerCase());
    if (dist < bestDist) {
      bestDist = dist;
      best = k;
    }
  }
  // Suggest only when the miss is small relative to the key length, so we do not
  // "correct" a genuinely new field to an unrelated one.
  return bestDist <= Math.max(2, Math.floor(key.length / 3)) ? best : null;
}

function levenshtein(a: string, b: string): number {
  const m = a.length;
  const n = b.length;
  const d: number[][] = Array.from({ length: m + 1 }, () => new Array(n + 1).fill(0));
  for (let i = 0; i <= m; i++) d[i][0] = i;
  for (let j = 0; j <= n; j++) d[0][j] = j;
  for (let i = 1; i <= m; i++) {
    for (let j = 1; j <= n; j++) {
      const cost = a[i - 1] === b[j - 1] ? 0 : 1;
      d[i][j] = Math.min(d[i - 1][j] + 1, d[i][j - 1] + 1, d[i - 1][j - 1] + cost);
    }
  }
  return d[m][n];
}
