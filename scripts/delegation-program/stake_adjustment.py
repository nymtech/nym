#!/usr/bin/env python3
"""
Delegation Program stake adjustment tool.

Takes a single-column CSV of NODE_IDs and returns a sheet of per-node stats plus
a SUGGESTED DELEGATION for each, so the first two columns can be fed straight
back into nym-cli:

  ./nym-cli mixnet delegators delegate-multi --mnemonic "<MNEMONIC>" --input <FILE>.csv

Delegation rules (all amounts in NYM):

  * Every suggested delegation is a multiple of 25k from a fixed ladder:
        0, 25k, 50k, 75k, 100k, 125k
  * The saturation point is read live from the chain, not hardcoded.
  * OVER the cap (default 90% of saturation): step DOWN the ladder until the
    resulting total stake fits under the cap.
  * UNDER the floor (default 60% of saturation): step UP the ladder as far as
    possible while staying under the cap.
  * BETWEEN floor and cap: leave the delegation untouched. The node is a good
    pick with headroom, which leaves room for organic delegators.
"""
import argparse
import csv
import sys
from pathlib import Path
import re

import requests
import pandas as pd

API_SPECTRE_ROOT = "https://api.nym.spectredao.net/api/v1"
API_VALIDATOR = "https://validator.nymtech.net/api/v1"
API_NS_DVPN = "https://mainnet-node-status-api.nymtech.cc/dvpn/v1/directory/gateways"
API_NS_V2 = "https://mainnet-node-status-api.nymtech.cc/v2/gateways"
API_REWARD_PARAMS = f"{API_VALIDATOR}/epoch/reward_params"

NYM_FACTOR = 1_000_000

# Delegations only ever take these values, in NYM.
DELEGATION_LADDER = [0, 25_000, 50_000, 75_000, 100_000, 125_000]

# Spectre ignores page/size style params and caps at 200 unless given `limit`.
SPECTRE_LIMIT = 10_000
# The NS API v2 caps page size at 200 whatever you ask for.
NS_PAGE_SIZE = 200


def parse_args():
    p = argparse.ArgumentParser(
        prog="stake_adjustment.py",
        description="Suggest wallet delegation adjustments per node against a live saturation point",
    )
    p.add_argument("input", help="Path to CSV with a single column of NODE_ID values")
    p.add_argument("--wallet_address", default="n1rnxpdpx3kldygsklfft0gech7fhfcux4zst5lw",
                   help="Delegation wallet address to track and adjust. Default: %(default)s")
    p.add_argument("--saturation", type=int, default=None,
                   help="Override stake saturation in NYM. Default: read live from the chain")
    p.add_argument("--stake_cap", type=int, default=90,
                   help="Upper bound as %% of saturation. Default: 90")
    p.add_argument("--stake_floor", type=int, default=60,
                   help="Below this %% of saturation the node is topped up. Default: 60")
    p.add_argument("--adjustment_step", type=int, default=25_000,
                   help="Ladder step in NYM. Default: 25000")
    p.add_argument("--max_wallet_delegation", type=int, default=125_000,
                   help="Maximum delegation allowed by the wallet in NYM. Default: 125000")
    p.add_argument("--denom", type=str, default="NYM", choices=["NYM", "uNYM", "nym", "unym"],
                   help="Output denomination. Default: NYM")
    p.add_argument("-o", "--output", default="./delegations_adjusted.csv",
                   help="Output CSV path. Default: %(default)s")
    p.add_argument("-y", "--assume-yes", action="store_true",
                   help="Save without asking")
    return p.parse_args()


# ── denomination helpers ────────────────────────────────────────────────────
def to_unym(value, denom: str) -> int:
    d = denom.lower()
    if d == "nym":
        return int(round(float(value) * NYM_FACTOR))
    if d == "unym":
        return int(value)
    raise ValueError("denom must be NYM or uNYM")


def from_unym(value_unym, denom: str) -> int:
    d = denom.lower()
    if d == "nym":
        return int(int(value_unym) // NYM_FACTOR)
    if d == "unym":
        return int(value_unym)
    raise ValueError("denom must be NYM or uNYM")


def _sanitize_text(val):
    """Collapse whitespace, remove control chars, strip pipes that break CSVs."""
    if val is None:
        return None
    s = str(val)
    s = s.replace("\r", " ").replace("\n", " ").replace("\t", " ")
    s = re.sub(r"\s+", " ", s)
    s = s.replace("|", " ")
    s = "".join(ch for ch in s if ch.isprintable())
    return s.strip()


def read_node_ids(csv_path: str) -> list:
    path = Path(csv_path)
    if not path.exists():
        raise RuntimeError(f"Input file not found: {csv_path}")

    node_ids = []
    with path.open(newline="") as f:
        for row in csv.reader(f):
            if not row:
                continue
            cell = row[0].strip()
            if not cell:
                continue
            # tolerate a header line
            if not cell.lstrip("-").isdigit():
                if len(node_ids) == 0:
                    continue
                raise RuntimeError(f"Invalid NODE_ID (not an integer): {cell!r}")
            node_ids.append(int(cell))
    if not node_ids:
        raise RuntimeError("Input CSV contains no NODE_IDs.")
    return node_ids


# ── fetching ────────────────────────────────────────────────────────────────
def _get_json(url, params=None, timeout=60):
    r = requests.get(url, params=params or {}, timeout=timeout,
                     headers={"User-Agent": "nym-stake-adjustment/2.0"})
    r.raise_for_status()
    return r.json()


def fetch_saturation_point_unym() -> int:
    """Live stake_saturation_point from the chain, in unym."""
    d = _get_json(API_REWARD_PARAMS, timeout=30)
    raw = d["interval"]["stake_saturation_point"]
    return int(float(raw))


def fetch_nodes_spectre() -> list:
    """Spectre ignores page/size and caps at 200 without an explicit limit."""
    data = _get_json(f"{API_SPECTRE_ROOT}/nodes", {"limit": SPECTRE_LIMIT}, timeout=90)
    return data if isinstance(data, list) else data.get("data", [])


def fetch_wallet_delegations(wallet: str) -> list:
    data = _get_json(f"{API_SPECTRE_ROOT}/delegations/{wallet}",
                     {"limit": SPECTRE_LIMIT}, timeout=60)
    return data if isinstance(data, list) else data.get("data", [])


def fetch_ns_gateways_v2() -> list:
    """NS API v2 - paginated, hard capped at 200 per page."""
    out, page = [], 0
    while True:
        d = _get_json(API_NS_V2, {"page": page, "size": NS_PAGE_SIZE}, timeout=90)
        items = d.get("items", [])
        out.extend(items)
        total = d.get("total", len(out))
        if len(out) >= total or not items or page > 50:
            break
        page += 1
    return out


def fetch_ns_dvpn_gateways() -> list:
    """dVPN directory - carries location.asn.kind (residential / other)."""
    d = _get_json(API_NS_DVPN, timeout=90)
    return d if isinstance(d, list) else d.get("data", [])


# ── delegation ladder ───────────────────────────────────────────────────────
def solve_delegation(current_total_nym: int, current_wallet_nym: int,
                     saturation_nym: int, cap_pct: int, floor_pct: int,
                     ladder: list) -> int:
    """Return the suggested wallet delegation in NYM, always a ladder value.

    base_nym is the node's stake excluding our own delegation, so swapping our
    delegation up or down moves the total by exactly that difference.
    """
    cap_nym = (saturation_nym * cap_pct) // 100
    floor_nym = (saturation_nym * floor_pct) // 100
    base_nym = current_total_nym - current_wallet_nym

    # Between floor and cap: leave it alone, there is room for human delegators.
    if floor_nym <= current_total_nym <= cap_nym:
        return current_wallet_nym

    allowed = sorted(ladder)

    if current_total_nym > cap_nym:
        # Oversaturated: step DOWN to the highest ladder value that fits.
        for value in reversed(allowed):
            if value > current_wallet_nym:
                continue  # never increase while reducing
            if base_nym + value <= cap_nym:
                return value
        return allowed[0]  # even 0 does not fit; nothing more we can do

    # Under the floor: step UP to the highest ladder value still under the cap.
    best = current_wallet_nym
    for value in allowed:
        if value < current_wallet_nym:
            continue  # never decrease while topping up
        if base_nym + value <= cap_nym:
            best = value
    return best


# ── per-node row ────────────────────────────────────────────────────────────
def build_row(node_id: int, saturation_unym: int, cap_pct: int, floor_pct: int,
              ladder_nym: list, out_denom: str, nodes_map: dict,
              wallet_map: dict, ns_map: dict, dvpn_map: dict) -> dict:
    m = nodes_map.get(node_id) or {}

    current_total_unym = int(m.get("total_stake") or 0)
    wallet_unym = int(wallet_map.get(node_id, 0))

    # ladder maths is done in whole NYM
    saturation_nym = saturation_unym // NYM_FACTOR
    current_total_nym = current_total_unym // NYM_FACTOR
    current_wallet_nym = wallet_unym // NYM_FACTOR

    suggested_wallet_nym = solve_delegation(
        current_total_nym, current_wallet_nym, saturation_nym,
        cap_pct, floor_pct, ladder_nym,
    )
    suggested_total_nym = current_total_nym - current_wallet_nym + suggested_wallet_nym

    suggested_sat = int((suggested_total_nym * 100) // (saturation_nym or 1))
    current_sat = int((current_total_nym * 100) // (saturation_nym or 1))

    # output denomination
    def out(nym_value):
        return int(nym_value) if out_denom.lower() == "nym" else int(nym_value) * NYM_FACTOR

    # ── spectre-derived metadata ──
    identity_key = m.get("identity_key")
    bonding_addr = m.get("bonding_address")
    uptime = m.get("uptime")
    accepted_tnc = m.get("accepted_tnc")
    config_score = m.get("config_score")

    desc = m.get("description") or {}
    build = desc.get("build_information") or {}
    binary_name = build.get("binary_name")
    version = build.get("build_version")

    role = None
    declared = desc.get("declared_role") or {}
    if declared:
        if declared.get("exit_ipr") or declared.get("exit_nr"):
            role = "exit-gateway"
        elif declared.get("entry"):
            role = "entry-gateway"
        elif declared.get("mixnode"):
            role = "mixnode"

    host_info = desc.get("host_information") or {}
    ip_list = host_info.get("ip_address")
    if isinstance(ip_list, list) and ip_list:
        ip_address = ip_list[0]
    elif isinstance(ip_list, str):
        ip_address = ip_list
    else:
        ip_address = None
    hostname = host_info.get("hostname")

    wss_port = (desc.get("mixnet_websockets") or {}).get("wss_port")

    wg = m.get("wireguard") or desc.get("wireguard")
    wg_enabled = bool(isinstance(wg, dict) and wg.get("port") and wg.get("public_key"))

    moniker = (m.get("self_description") or {}).get("moniker")

    # cost params
    cost = ((m.get("rewarding_details") or {}).get("cost_params") or {})
    pm_raw = cost.get("profit_margin_percent")
    try:
        profit_margin = f"{float(pm_raw) * 100:.2f}%" if pm_raw is not None else None
    except (TypeError, ValueError):
        profit_margin = None
    oc_raw = (cost.get("interval_operating_cost") or {}).get("amount")
    try:
        operating_cost = from_unym(int(oc_raw), out_denom) if oc_raw is not None else None
    except (TypeError, ValueError):
        operating_cost = None

    # ── NS API v2 probe-derived fields, keyed by identity ──
    ns = ns_map.get(identity_key) or {}
    outcome = ((ns.get("last_probe_result") or {}).get("outcome") or {})
    wgo = outcome.get("wg") or {}
    lpo = outcome.get("lp") or {}

    # "did WireGuard even connect" rather than a throughput number
    if wgo:
        wg_performance = bool(wgo.get("can_handshake_v4") or wgo.get("can_handshake_v6"))
    else:
        wg_performance = None

    lp_enabled = bool(lpo.get("can_connect")) if lpo else None

    ipv6 = wgo.get("can_handshake_v6") if wgo else None
    dns_v4 = wgo.get("can_resolve_dns_v4") if wgo else None
    dns_v6 = wgo.get("can_resolve_dns_v6") if wgo else None

    # ── dVPN directory: location kind (residential / other) ──
    dv = dvpn_map.get(identity_key) or {}
    location_kind = (((dv.get("location") or {}).get("asn") or {}).get("kind"))

    # ── client account balance sufficiency ──
    # There is no public endpoint exposing a node's client-account balance, and
    # nym-api does not surface `has_sufficient_tokens` directly. What it does
    # expose is the effect: a node that cannot send transactions has its
    # config_score multiplied by 0.8 (see the config score documentation).
    #
    # A score of exactly 1.0 is unpenalised. Anything at or below 0.8 that
    # divides back to a plausible unpenalised score (<= 1.0) carries the penalty.
    # Scores that are effectively zero are zeroed by a different factor (wrong
    # binary, T&C not accepted, self-described API down), so we report None.
    client_sufficient = None
    if isinstance(config_score, (int, float)):
        if config_score >= 0.99:
            client_sufficient = True
        elif config_score < 1e-6:
            client_sufficient = None       # zeroed by another factor entirely
        elif config_score <= 0.8 + 1e-9:
            client_sufficient = False      # sits on the 0.8-penalised grid
        else:
            client_sufficient = True       # version-penalised only, balance fine

    return {
        "NODE ID": node_id,
        "SUGGESTED DELEGATION": out(suggested_wallet_nym),
        "CURRENT DELEGATION": out(current_wallet_nym),
        "SUGGESTED TOTAL STAKE": out(suggested_total_nym),
        "CURRENT TOTAL STAKE": out(current_total_nym),
        "SUGGESTED SATURATION": suggested_sat,
        "CURRENT SATURATION": current_sat,
        "UPTIME": uptime,
        "VERSION": _sanitize_text(version),
        "T&C": bool(accepted_tnc) if accepted_tnc is not None else None,
        "BINARY": _sanitize_text(binary_name),
        "ROLE": _sanitize_text(role),
        "WIREGUARD": wg_enabled,
        "PROFIT MARGIN": profit_margin,
        "OPERATING COST": operating_cost,
        "WG CONNECTED": wg_performance,
        "LP ENABLED": lp_enabled,
        "CONFIG SCORE": config_score,
        "IPV6": ipv6,
        "DNS V4": dns_v4,
        "DNS V6": dns_v6,
        "LOCATION KIND": _sanitize_text(location_kind),
        "CLIENT SUFFICIENT BALANCE": client_sufficient,
        "IP ADDRESS": _sanitize_text(ip_address),
        "HOSTNAME": _sanitize_text(hostname),
        "WSS PORT": wss_port,
        "MONIKER": _sanitize_text(moniker),
        "IDENTITY KEY": _sanitize_text(identity_key),
        "BONDING WALLET": _sanitize_text(bonding_addr),
        "EXPLORER URL": f"https://explorer.nym.spectredao.net/nodes/{identity_key}" if identity_key else "",
    }


def main():
    args = parse_args()
    denom = args.denom

    node_ids = read_node_ids(args.input)

    dups = pd.Series(node_ids).duplicated(keep=False)
    if dups.any():
        dup_ids = sorted({nid for nid, d in zip(node_ids, dups.tolist()) if d})
        print(f"warning: These node IDs are duplicated: {dup_ids}")
    else:
        print("There are no duplicated node IDs.")

    # saturation point: live unless overridden
    if args.saturation is not None:
        saturation_unym = to_unym(args.saturation, denom)
        print(f"* * * Using saturation override: {saturation_unym // NYM_FACTOR:,} NYM * * *")
    else:
        print("* * * Fetching live stake saturation point * * *")
        saturation_unym = fetch_saturation_point_unym()
        print(f"    stake_saturation_point: {saturation_unym // NYM_FACTOR:,} NYM")

    # ladder capped at the wallet maximum
    ladder_nym = [v for v in DELEGATION_LADDER if v <= args.max_wallet_delegation]
    if args.adjustment_step != 25_000:
        ladder_nym = list(range(0, args.max_wallet_delegation + 1, args.adjustment_step))
    print(f"    delegation ladder (NYM): {ladder_nym}")
    print(f"    cap {args.stake_cap}%  floor {args.stake_floor}%")

    print("* * * Fetching wallet delegations * * *")
    wallet_map = {}
    for d in fetch_wallet_delegations(args.wallet_address):
        try:
            nid = int(d.get("node_id"))
            amt = int((d.get("amount") or {}).get("amount"))
        except (TypeError, ValueError):
            continue
        wallet_map[nid] = wallet_map.get(nid, 0) + amt
    print(f"    {len(wallet_map)} delegated node(s)")

    print("* * * Fetching nodes (Spectre) * * *")
    nodes_map = {}
    for n in fetch_nodes_spectre():
        try:
            nodes_map[int(n.get("node_id"))] = n
        except (TypeError, ValueError):
            continue
    print(f"    {len(nodes_map)} node(s)")

    print("* * * Fetching probe results (NS API v2) * * *")
    ns_map = {}
    try:
        for g in fetch_ns_gateways_v2():
            k = g.get("gateway_identity_key")
            if k:
                ns_map[k] = g
    except Exception as e:
        print(f"    warning: probe data unavailable: {e}", file=sys.stderr)
    print(f"    {len(ns_map)} gateway(s)")

    print("* * * Fetching location kinds (dVPN directory) * * *")
    dvpn_map = {}
    try:
        for g in fetch_ns_dvpn_gateways():
            k = g.get("identity_key")
            if k:
                dvpn_map[k] = g
    except Exception as e:
        print(f"    warning: location data unavailable: {e}", file=sys.stderr)
    print(f"    {len(dvpn_map)} gateway(s)")

    rows = []
    for nid in node_ids:
        try:
            rows.append(build_row(
                node_id=nid,
                saturation_unym=saturation_unym,
                cap_pct=args.stake_cap,
                floor_pct=args.stake_floor,
                ladder_nym=ladder_nym,
                out_denom=denom,
                nodes_map=nodes_map,
                wallet_map=wallet_map,
                ns_map=ns_map,
                dvpn_map=dvpn_map,
            ))
        except Exception as e:
            print(f"warning: node {nid}: {e}", file=sys.stderr)
            rows.append({"NODE ID": nid})

    df = pd.DataFrame(rows)

    print("\nResult preview:")
    print(df.to_string(index=False))

    # summary of what would change
    if "SUGGESTED DELEGATION" in df and "CURRENT DELEGATION" in df:
        changed = df[df["SUGGESTED DELEGATION"] != df["CURRENT DELEGATION"]]
        up = changed[changed["SUGGESTED DELEGATION"] > changed["CURRENT DELEGATION"]]
        down = changed[changed["SUGGESTED DELEGATION"] < changed["CURRENT DELEGATION"]]
        print(f"\nChanges: {len(changed)} node(s)  |  top up: {len(up)}  reduce: {len(down)}  "
              f"unchanged: {len(df) - len(changed)}")

    if args.assume_yes:
        ans = "y"
    else:
        ans = input(f"\nSave to {args.output} ? [y/N]: ").strip().lower()
    if ans == "y":
        out_path = Path(args.output)
        df.to_csv(out_path, index=False)
        print(f"Saved: {out_path.resolve()}")
    else:
        print("Not saved.")


if __name__ == "__main__":
    main()