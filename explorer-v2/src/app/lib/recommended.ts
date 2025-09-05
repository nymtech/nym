// return the top 10 node_ids matching the constraints, computed at runtime.
export async function fetchRecommendedNodes(): Promise<number[]> {
  const url = "https://api.nym.spectredao.net/api/v1/nodes?size=3000";

  const res = await fetch(url, { cache: "no-store" });
  if (!res.ok) throw new Error(`Failed to fetch nodes: ${res.status}`);
  const data: any[] = await res.json();

  const MIN_STAKE = 50_000_000_000; // 50B

  const filtered = data.filter((n) => {
    const ws9000 = n?.description?.mixnet_websockets?.ws_port === 9000;
    const wgOn = n?.description?.wireguard != null;
    const pm = Number(n?.rewarding_details?.cost_params?.profit_margin_percent ?? "1");
    const pmOk = !Number.isNaN(pm) && pm <= 0.2;

    const role = n?.description?.declared_role ?? {};
    const rolesOk = !!(role.entry && role.exit_ipr && role.exit_nr);

    const stakeOk = Number(n?.total_stake ?? 0) > MIN_STAKE;

    return ws9000 && wgOn && pmOk && rolesOk && stakeOk;
  });

  filtered.sort((a, b) => {
    const ua = Number(a?.uptime ?? 0), ub = Number(b?.uptime ?? 0);
    if (ub !== ua) return ub - ua; // uptime DESC
    const sa = Number(a?.total_stake ?? 0), sb = Number(b?.total_stake ?? 0);
    return sa - sb; // stake ASC
  });

  return filtered.slice(0, 10).map((n) => Number(n?.node_id));
}
