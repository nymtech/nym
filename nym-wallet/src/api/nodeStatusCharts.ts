import type { GatewayProbeOutcome } from './nodeStatus';

/** Measures from WireGuard probe: either download latency (ms) or ping success (0-100 %). */
export type WgMeasureBar = { name: string; value: number; kind: 'milliseconds' | 'percent' };

/** Per probe category: percent of checks that passed (0-100). */
export type ProbeGroupBar = { name: string; pctPassed: number; passed: number; total: number };

export function probeGroupsForChart(outcome: GatewayProbeOutcome | undefined | null): ProbeGroupBar[] {
  const groups: ProbeGroupBar[] = [];

  const pushGroup = (name: string, p: number, t: number) => {
    if (t === 0) {
      return;
    }
    groups.push({
      name,
      passed: p,
      total: t,
      pctPassed: Math.round((p / t) * 100),
    });
  };

  if (outcome?.as_entry) {
    const e = outcome.as_entry;
    let p = 0;
    let t = 0;
    if (typeof e.can_connect === 'boolean') {
      t += 1;
      if (e.can_connect) {
        p += 1;
      }
    }
    if (typeof e.can_route === 'boolean') {
      t += 1;
      if (e.can_route) {
        p += 1;
      }
    }
    pushGroup('Entry', p, t);
  }

  if (outcome?.as_exit) {
    const x = outcome.as_exit;
    const keys = [
      'can_connect',
      'can_route_ip_v4',
      'can_route_ip_v6',
      'can_route_ip_external_v4',
      'can_route_ip_external_v6',
    ] as const;
    const exitCounts = keys.reduce(
      (acc, k) => {
        const v = x[k];
        if (typeof v === 'boolean') {
          return { p: acc.p + (v ? 1 : 0), t: acc.t + 1 };
        }
        return acc;
      },
      { p: 0, t: 0 },
    );
    pushGroup('Exit', exitCounts.p, exitCounts.t);
  }

  if (outcome?.socks5) {
    const s = outcome.socks5;
    let p = 0;
    let t = 0;
    if (typeof s.can_connect_socks5 === 'boolean') {
      t += 1;
      if (s.can_connect_socks5) {
        p += 1;
      }
    }
    if (s.https_connectivity && typeof s.https_connectivity.https_success === 'boolean') {
      t += 1;
      if (s.https_connectivity.https_success) {
        p += 1;
      }
    }
    pushGroup('SOCKS5', p, t);
  }

  if (outcome?.wg) {
    const w = outcome.wg;
    const boolKeys = [
      'can_handshake_v4',
      'can_handshake_v6',
      'can_query_metadata_v4',
      'can_register',
      'can_resolve_dns_v4',
      'can_resolve_dns_v6',
    ] as const;
    const wgCounts = boolKeys.reduce(
      (acc, k) => {
        const v = w[k];
        if (typeof v === 'boolean') {
          return { p: acc.p + (v ? 1 : 0), t: acc.t + 1 };
        }
        return acc;
      },
      { p: 0, t: 0 },
    );
    pushGroup('WireGuard', wgCounts.p, wgCounts.t);
  }

  if (outcome?.lp) {
    const l = outcome.lp;
    const lpKeys = ['can_connect', 'can_handshake', 'can_register'] as const;
    const lpCounts = lpKeys.reduce(
      (acc, k) => {
        const v = l[k];
        if (typeof v === 'boolean') {
          return { p: acc.p + (v ? 1 : 0), t: acc.t + 1 };
        }
        return acc;
      },
      { p: 0, t: 0 },
    );
    pushGroup('LP', lpCounts.p, lpCounts.t);
  }

  return groups;
}

/** Bars for WG download latency (ms) or ping success (percent). Only one family is returned per probe. */
export function wgComparisonBars(outcome: GatewayProbeOutcome | undefined | null): WgMeasureBar[] {
  const w = outcome?.wg;
  if (!w) {
    return [];
  }
  const v4 = w.download_duration_milliseconds_v4;
  const v6 = w.download_duration_milliseconds_v6;
  if (typeof v4 === 'number' || typeof v6 === 'number') {
    const out: WgMeasureBar[] = [];
    if (typeof v4 === 'number') {
      out.push({ name: 'IPv4 download', value: v4, kind: 'milliseconds' });
    }
    if (typeof v6 === 'number') {
      out.push({ name: 'IPv6 download', value: v6, kind: 'milliseconds' });
    }
    return out;
  }
  const p4 = w.ping_ips_performance_v4;
  const p6 = w.ping_ips_performance_v6;
  const out: WgMeasureBar[] = [];
  if (typeof p4 === 'number') {
    out.push({ name: 'IPv4 ping success', value: Math.round(p4 * 100), kind: 'percent' });
  }
  if (typeof p6 === 'number') {
    out.push({ name: 'IPv6 ping success', value: Math.round(p6 * 100), kind: 'percent' });
  }
  return out;
}

export function socks5LatencyMs(outcome: GatewayProbeOutcome | undefined | null): number | undefined {
  const ms = outcome?.socks5?.https_connectivity?.https_latency_ms;
  return typeof ms === 'number' ? ms : undefined;
}
