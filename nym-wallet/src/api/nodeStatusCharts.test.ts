import { probeGroupsForChart, socks5LatencyMs, wgComparisonBars } from './nodeStatusCharts';

describe('probeGroupsForChart', () => {
  it('returns empty array when outcome is missing', () => {
    expect(probeGroupsForChart(undefined)).toStrictEqual([]);
    expect(probeGroupsForChart(null)).toStrictEqual([]);
  });

  it('aggregates entry booleans', () => {
    expect(
      probeGroupsForChart({
        as_entry: { can_connect: true, can_route: false },
      }),
    ).toStrictEqual([{ name: 'Entry', passed: 1, total: 2, pctPassed: 50 }]);
  });

  it('skips entry group when no boolean fields are present', () => {
    expect(probeGroupsForChart({ as_entry: {} })).toStrictEqual([]);
  });

  it('aggregates exit routing flags', () => {
    const groups = probeGroupsForChart({
      as_exit: {
        can_connect: true,
        can_route_ip_v4: false,
        can_route_ip_v6: true,
      },
    });
    expect(groups).toContainEqual({ name: 'Exit', passed: 2, total: 3, pctPassed: 67 });
  });

  it('includes SOCKS5 when connectivity flags exist', () => {
    expect(
      probeGroupsForChart({
        socks5: {
          can_connect_socks5: true,
          https_connectivity: { https_success: false, https_latency_ms: 12 },
        },
      }),
    ).toStrictEqual([{ name: 'SOCKS5', passed: 1, total: 2, pctPassed: 50 }]);
  });

  it('includes WireGuard and LP groups from booleans', () => {
    const groups = probeGroupsForChart({
      wg: { can_handshake_v4: true, can_handshake_v6: false },
      lp: { can_connect: true, can_handshake: true, can_register: false },
    });
    expect(groups).toContainEqual({ name: 'WireGuard', passed: 1, total: 2, pctPassed: 50 });
    expect(groups).toContainEqual({ name: 'LP', passed: 2, total: 3, pctPassed: 67 });
  });
});

describe('wgComparisonBars', () => {
  it('prefers download duration bars when present', () => {
    expect(
      wgComparisonBars({
        wg: { download_duration_milliseconds_v4: 120, download_duration_milliseconds_v6: 200 },
      }),
    ).toStrictEqual([
      { name: 'IPv4 download', value: 120, kind: 'milliseconds' },
      { name: 'IPv6 download', value: 200, kind: 'milliseconds' },
    ]);
  });

  it('falls back to ping performance as percent', () => {
    expect(
      wgComparisonBars({
        wg: { ping_ips_performance_v4: 0.88, ping_ips_performance_v6: 0.5 },
      }),
    ).toStrictEqual([
      { name: 'IPv4 ping success', value: 88, kind: 'percent' },
      { name: 'IPv6 ping success', value: 50, kind: 'percent' },
    ]);
  });

  it('returns empty when wg is absent', () => {
    expect(wgComparisonBars(undefined)).toStrictEqual([]);
  });
});

describe('socks5LatencyMs', () => {
  it('returns latency when numeric', () => {
    expect(
      socks5LatencyMs({
        socks5: { https_connectivity: { https_latency_ms: 42 } },
      }),
    ).toBe(42);
  });

  it('returns undefined when missing or non-numeric', () => {
    expect(socks5LatencyMs(undefined)).toBeUndefined();
    expect(socks5LatencyMs({ socks5: {} })).toBeUndefined();
  });
});
