#!/usr/bin/env python3
import argparse
import csv
import sys
from pathlib import Path
import requests
import pandas as pd
import re

API_SPECTRE_ROOT = "https://api.nym.spectredao.net/api/v1"
API_VALIDATOR    = "https://validator.nymtech.net/api/v1"
API_BASE         = f"{API_SPECTRE_ROOT}/nodes"
NYM_FACTOR = 1_000_000

"""
This simple argument based program is designed primarily for Delegation program management.
The main goal is to generate a csv of which first two columns without headers can also be reused for nym-cli as input:
./nym-cli mixnet delegators delegate-multi --mnemonic "<MNEMONIC>" --input <PATH>/<FILE>.csv

The default values therefore are:
--wallet_address: Nym Team DP wallet address
--saturation: 250k NYM
--stake_cap: 90% as per DP rules
--adjustment_Step: 25k NYM as per DP rules
--max_wallet_delegation: 125k NYM as per DP rules
--denom: NYM not uNYM to make it smoother and aligned with delegate-multi command of nym-cli

Additionaly the program scrapes described endpoint and returns a sheet with 20 values per node. Those are:
NODE ID, SUGGESTED WALLET DELEGATION, CURRENT WALLET DELEGATION, SUGGESTED TOTAL STAKE,	CURRENT TOTAL STAKE,
SUGGESTED SATURATION, CURRENT SATURATION, UPTIME, VERSION, T&C, BINARY, ROLE, WIREGUARD, IP ADDRESS, HOSTNAME,
WSS PORT, MONIKER, IDENTITY KEY, BONDING WALLET, EXPLORER URL.
"""

def parse_args():
    p = argparse.ArgumentParser(
        prog="stake_adjustment.py",
        description="Suggest wallet delegation adjustments per node to hit a target saturation cap",
    )
    p.add_argument("input", help="Path to CSV with a single column of NODE_ID values")
    p.add_argument("--saturation", type=int, default=250_000,
                   help="Stake saturation in NYM (or uNYM if --denom uNYM). Default: 250000")
    p.add_argument("--wallet_address", default="n1rnxpdpx3kldygsklfft0gech7fhfcux4zst5lw",
                   help="Delegation wallet address to track and adjust. Default: %(default)s")
    p.add_argument("--stake_cap", type=int, default=90,
                   help="Target percentage of max saturation (e.g., 90 for 90%%). Default: 90")
    p.add_argument("--adjustment_step", type=int, default=25_000,
                   help="Amount to undelegate per step (NYM or uNYM) until target delegation percentage is met. Default: 25000")
    p.add_argument("--max_wallet_delegation", type=int, default=125_000,
                   help="Maximum delegation allowed by the wallet (NYM or uNYM). Default: 125000")
    p.add_argument("--denom", type=str, default="NYM", choices=["NYM", "uNYM", "nym", "unym"],
                   help="Input/output denomination. Default: NYM")
    return p.parse_args()


def to_unym(value: int, denom: str) -> int:
    d = denom.lower()
    if d == "nym":
        return int(value) * NYM_FACTOR
    if d == "unym":
        return int(value)
    raise ValueError("denom must be NYM or uNYM")


def from_unym(value_unym: int, denom: str) -> int:
    d = denom.lower()
    if d == "nym":
        return int(value_unym // NYM_FACTOR)
    if d == "unym":
        return int(value_unym)
    raise ValueError("denom must be NYM or uNYM")


def read_node_ids(csv_path: str) -> list[int]:
    path = Path(csv_path)
    if not path.exists():
        raise RuntimeError(f"Input file not found: {csv_path}")

    node_ids: list[int] = []
    with path.open(newline="") as f:
        reader = csv.reader(f)
        for row in reader:
            if not row:
                continue
            if len(row) != 1:
                raise RuntimeError("Input CSV must have exactly one column of NODE_ID values.")
            try:
                node_ids.append(int(row[0]))
            except ValueError:
                raise RuntimeError(f"Invalid NODE_ID (not an integer): {row[0]!r}")
    if not node_ids:
        raise RuntimeError("Input CSV contains no NODE_IDs.")
    return node_ids


# pagination helpers: limit/offset -> then page/page_size -> single shot fallback
def _fetch_all_limit_offset(url: str, limit: int = 1000, timeout: int = 60) -> list:
    items = []
    offset = 0
    seen_guard = None
    loops = 0
    while True:
        loops += 1
        r = requests.get(url, params={"limit": limit, "offset": offset}, timeout=timeout)
        if r.status_code >= 400:
            return None
        try:
            data = r.json()
        except Exception:
            return None
        if isinstance(data, dict) and "data" in data and isinstance(data["data"], list):
            batch = data["data"]
        elif isinstance(data, list):
            batch = data
        else:
            return None
        if not batch:
            break
        items.extend(batch)
        if len(batch) < limit:
            break
        # guard against APIs that ignore offset
        first_sig = str(batch[0])
        if first_sig == seen_guard:
            break
        seen_guard = first_sig
        offset += len(batch)
        if loops > 1000 or offset > 1_000_000:
            break
    return items


def _fetch_all_page_pagesize(url: str, page_size: int = 1000, timeout: int = 60) -> list:
    items = []
    page = 1
    seen_guard = None
    loops = 0
    while True:
        loops += 1
        r = requests.get(url, params={"page": page, "page_size": page_size}, timeout=timeout)
        if r.status_code >= 400:
            return None
        try:
            data = r.json()
        except Exception:
            return None
        if isinstance(data, dict) and "data" in data and isinstance(data["data"], list):
            batch = data["data"]
        elif isinstance(data, list):
            batch = data
        else:
            return None
        if not batch:
            break
        items.extend(batch)
        if len(batch) < page_size:
            break
        first_sig = str(batch[0])
        if first_sig == seen_guard:
            break
        seen_guard = first_sig
        page += 1
        if loops > 1000 or page > 10000:
            break
    return items


def _fetch_all_any(url: str, timeout: int = 60) -> list:
    # try limit/offset
    got = _fetch_all_limit_offset(url, limit=1000, timeout=timeout)
    if isinstance(got, list) and got:
        return got
    # try page/page_size
    got = _fetch_all_page_pagesize(url, page_size=1000, timeout=timeout)
    if isinstance(got, list) and got:
        return got
    # fallback: single shot
    r = requests.get(url, timeout=timeout)
    r.raise_for_status()
    data = r.json()
    if isinstance(data, dict) and "data" in data and isinstance(data["data"], list):
        return data["data"]
    if isinstance(data, list):
        return data
    raise RuntimeError(f"Unexpected response format from {url}")


# fetching functions using robust pagination
def fetch_wallet_delegations(wallet: str) -> list[dict]:
    url = f"{API_SPECTRE_ROOT}/delegations/{wallet}"
    return _fetch_all_any(url, timeout=45)


def fetch_nodes_spectre() -> list[dict]:
    url = f"{API_SPECTRE_ROOT}/nodes"
    return _fetch_all_any(url, timeout=60)


def fetch_nodes_validator() -> list[dict]:
    url = f"{API_VALIDATOR}/nym-nodes/described"
    return _fetch_all_any(url, timeout=60)


def fetch_node_delegations_sum_unym(node_id: int) -> int:
    url = f"{API_SPECTRE_ROOT}/nodes/{node_id}/delegations"
    try:
        data = _fetch_all_any(url, timeout=45)
    except Exception:
        # last resort
        r = requests.get(url, timeout=45)
        r.raise_for_status()
        data = r.json()
    if not isinstance(data, list):
        return 0
    total = 0
    for item in data:
        try:
            total += int(item.get("amount", {}).get("amount"))
        except Exception:
            pass
    return total


def suggest_wallet_delegation(
    node_id: int,
    wallet: str,
    saturation_unym: int,
    cap_pct: int,
    step_unym: int,
    max_wallet_unym: int,
    out_denom: str,
    nodes_map: dict[int, dict],
    val_map: dict[int, dict],
    wallet_map: dict[int, int],
) -> dict:
    # CURRENT TOTAL STAKE (in uNYM)
    current_total_unym = None
    meta_src = None

    if node_id in nodes_map and isinstance(nodes_map[node_id], dict):
        # spectre nodes
        current_total_unym = int(nodes_map[node_id].get("total_stake") or 0)
        meta_src = nodes_map[node_id]
    elif node_id in val_map and isinstance(val_map[node_id], dict):
        # validator described
        current_total_unym = int(val_map[node_id].get("total_stake") or 0)
        meta_src = val_map[node_id]

    # if still unknown, sum delegations as a fallback
    if current_total_unym is None or current_total_unym == 0:
        current_total_unym = fetch_node_delegations_sum_unym(node_id)

    # CURRENT WALLET DELEGATION (in uNYM) from wallet_map
    wallet_unym = int(wallet_map.get(node_id, 0))

    # target cap in uNYM
    target_unym = (saturation_unym * cap_pct) // 100

    # start from min(current_wallet, max_wallet) and back off by step until under target
    suggested_wallet_unym = min(wallet_unym, max_wallet_unym)
    suggested_total_unym = current_total_unym - wallet_unym + suggested_wallet_unym

    if suggested_total_unym > target_unym and step_unym > 0:
        while suggested_total_unym > target_unym and suggested_wallet_unym > 0:
            dec = min(step_unym, suggested_wallet_unym)
            suggested_wallet_unym -= dec
            suggested_total_unym -= dec

    # convert to denom for output
    suggested_wallet = from_unym(suggested_wallet_unym, out_denom)
    current_wallet = from_unym(wallet_unym, out_denom)
    suggested_total = from_unym(suggested_total_unym, out_denom)
    current_total = from_unym(current_total_unym, out_denom)
    saturation_val = from_unym(saturation_unym, out_denom)

    # saturation as integer percentages
    suggested_sat = int((suggested_total * 100) // (saturation_val or 1))
    current_sat   = int((current_total * 100) // (saturation_val or 1))

    # extra fields
    uptime      = None
    version     = None
    accepted_tnc= None
    binary_name  = _sanitize_text(binary_name)
    role         = _sanitize_text(role)
    ip_address   = _sanitize_text(ip_address)
    hostname     = _sanitize_text(hostname)
    moniker      = _sanitize_text(moniker)
    identity_key = _sanitize_text(identity_key)
    bonding_addr = _sanitize_text(bonding_addr)
    explorer_url = _sanitize_text(explorer_url)
    version      = _sanitize_text(version)

    m = meta_src or {}

def suggest_wallet_delegation(
    node_id: int,
    wallet: str,
    saturation_unym: int,
    cap_pct: int,
    step_unym: int,
    max_wallet_unym: int,
    out_denom: str,
    # extra maps for node
    nodes_map: dict[int, dict],
    val_map: dict[int, dict],
    wallet_map: dict[int, int],
) -> dict:
    # CURRENT TOTAL STAKE (in uNYM)
    current_total_unym = None
    meta_src = None

    if node_id in nodes_map and isinstance(nodes_map[node_id], dict):
        # spectre nodes
        current_total_unym = int(nodes_map[node_id].get("total_stake") or 0)
        meta_src = nodes_map[node_id]
    elif node_id in val_map and isinstance(val_map[node_id], dict):
        # validator described
        current_total_unym = int(val_map[node_id].get("total_stake") or 0)
        meta_src = val_map[node_id]

    # if still unknown, sum delegations as a fallback
    if current_total_unym is None or current_total_unym == 0:
        current_total_unym = fetch_node_delegations_sum_unym(node_id)

    # CURRENT WALLET DELEGATION (in uNYM) from wallet_map
    wallet_unym = int(wallet_map.get(node_id, 0))

    # target cap in uNYM
    target_unym = (saturation_unym * cap_pct) // 100

    # start from min(current_wallet, max_wallet) and back off by step until under target
    suggested_wallet_unym = min(wallet_unym, max_wallet_unym)
    suggested_total_unym = current_total_unym - wallet_unym + suggested_wallet_unym

    if suggested_total_unym > target_unym and step_unym > 0:
        while suggested_total_unym > target_unym and suggested_wallet_unym > 0:
            dec = min(step_unym, suggested_wallet_unym)
            suggested_wallet_unym -= dec
            suggested_total_unym -= dec

    # convert to denom for output
    suggested_wallet = from_unym(suggested_wallet_unym, out_denom)
    current_wallet   = from_unym(wallet_unym, out_denom)
    suggested_total  = from_unym(suggested_total_unym, out_denom)
    current_total    = from_unym(current_total_unym, out_denom)
    saturation_val   = from_unym(saturation_unym, out_denom)

    # saturation as integer percentages
    suggested_sat = int((suggested_total * 100) // (saturation_val or 1))
    current_sat   = int((current_total * 100) // (saturation_val or 1))

    # extra fields
    uptime        = None
    version       = None
    accepted_tnc  = None
    binary_name   = None
    role          = None
    wg_enabled    = None
    ip_address    = None
    hostname      = None
    wss_port      = None
    moniker       = None
    identity_key  = None
    bonding_addr  = None
    explorer_url  = None

    m = meta_src or {}
    # spectre and validator conventions translation
    identity_key = m.get("identity_key")
    bonding_addr = m.get("bonding_address")
    uptime       = m.get("uptime")
    accepted_tnc = m.get("accepted_tnc")
    desc  = m.get("description") or {}
    build = desc.get("build_information") or {}
    binary_name = build.get("binary_name")
    version     = build.get("build_version")

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
    hostname = host_info.get("hostname")

    # wss/ws ports under mixnet_websockets
    webs = desc.get("mixnet_websockets") or {}
    wss_port = webs.get("wss_port")

    # wireguard info
    wg = m.get("wireguard") or desc.get("wireguard")
    if isinstance(wg, dict) and wg.get("port") and wg.get("public_key"):
        wg_enabled = True
    else:
        wg_enabled = False

    # self desc to get moniker
    self_desc = m.get("self_description") or {}
    moniker = self_desc.get("moniker")

    explorer_url = f"https://explorer.nym.spectredao.net/nodes/{identity_key}" if identity_key else ""

    # sanitize all string-ish fields to kill tabs/newlines and weird spacing
    binary_name  = _sanitize_text(binary_name)
    role         = _sanitize_text(role)
    ip_address   = _sanitize_text(ip_address)
    hostname     = _sanitize_text(hostname)
    moniker      = _sanitize_text(moniker)
    identity_key = _sanitize_text(identity_key)
    bonding_addr = _sanitize_text(bonding_addr)
    explorer_url = _sanitize_text(explorer_url)
    version      = _sanitize_text(version)

    return {
        "NODE ID": node_id,
        "SUGGESTED WALLET DELEGATION": suggested_wallet,
        "CURRENT WALLET DELEGATION": current_wallet,
        "SUGGESTED TOTAL STAKE": suggested_total,
        "CURRENT TOTAL STAKE": current_total,
        "SUGGESTED SATURATION": suggested_sat,
        "CURRENT SATURATION": current_sat,
        "UPTIME": uptime,
        "VERSION": version,
        "T&C": bool(accepted_tnc) if accepted_tnc is not None else None,
        "BINARY": binary_name,
        "ROLE": role,
        "WIREGUARD": wg_enabled,
        "IP ADDRESS": ip_address,
        "HOSTNAME": hostname,
        "WSS PORT": wss_port,
        "MONIKER": moniker,
        "IDENTITY KEY": identity_key,
        "BONDING WALLET": bonding_addr,
        "EXPLORER URL": explorer_url,
    }

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

def main():
    args = parse_args()
    denom = args.denom

    # convert user inputs to uNYM
    saturation_unym = to_unym(args.saturation, denom)
    step_unym = to_unym(args.adjustment_step, denom)
    max_wallet_unym = to_unym(args.max_wallet_delegation, denom)

    node_ids = read_node_ids(args.input)

    # detect duplicates (just report, do not modify order)
    dups = pd.Series(node_ids).duplicated(keep=False)
    if dups.any():
        dup_ids = sorted(set([nid for nid, d in zip(node_ids, dups.tolist()) if d]))
        print(f"warning: These node IDs are duplicated: {dup_ids}")
    else:
        print("There are no duplicated node IDs.")

    # one call for wallet delegations (then map node_id -> wallet amount)
    print("* * * Fetching wallet delegations * * *")
    wallet_delegs = fetch_wallet_delegations(args.wallet_address)
    wallet_map: dict[int, int] = {}
    for d in wallet_delegs:
        try:
            nid = int(d.get("node_id"))
            amt = int((d.get("amount") or {}).get("amount"))
        except Exception:
            continue
        wallet_map[nid] = wallet_map.get(nid, 0) + amt

    # pull nodes from Spectre (paginated)
    print("* * * Fetching nodes (Spectre) with pagination * * *")
    spectre_nodes = fetch_nodes_spectre()
    nodes_map: dict[int, dict] = {}
    for n in spectre_nodes:
        try:
            nid = int(n.get("node_id"))
        except Exception:
            continue
        nodes_map[nid] = n

    # pull nodes from Validator (paginated)
    print("* * * Fetching nodes (Validator) with pagination * * *")
    validator_nodes = fetch_nodes_validator()
    val_map: dict[int, dict] = {}
    for n in validator_nodes:
        try:
            nid = int(n.get("node_id"))
        except Exception:
            continue
        # do not overwrite spectre if present; keep as fallback
        if nid not in nodes_map:
            val_map[nid] = n

    # build rows
    rows = []
    for nid in node_ids:
        try:
            row = suggest_wallet_delegation(
                node_id=nid,
                wallet=args.wallet_address,
                saturation_unym=saturation_unym,
                cap_pct=args.stake_cap,
                step_unym=step_unym,
                max_wallet_unym=max_wallet_unym,
                out_denom=denom,
                nodes_map=nodes_map,
                val_map=val_map,
                wallet_map=wallet_map,
            )
        except Exception as e:
            row = {
                "NODE ID": nid,
                "SUGGESTED WALLET DELEGATION": 0,
                "CURRENT WALLET DELEGATION": 0,
                "SUGGESTED TOTAL STAKE": 0,
                "CURRENT TOTAL STAKE": 0,
                "SUGGESTED SATURATION": 0,
                "CURRENT SATURATION": 0,
                "UPTIME": None,
                "VERSION": None,
                "T&C": None,
                "BINARY": None,
                "ROLE": None,
                "WIREGUARD": None,
                "IP ADDRESS": None,
                "HOSTNAME": None,
                "WSS PORT": None,
                "MONIKER": None,
                "IDENTITY KEY": None,
                "BONDING WALLET": None,
                "EXPLORER URL": "",
            }
            print(f"warning: node {nid}: {e}", file=sys.stderr)
        rows.append(row)

    df = pd.DataFrame(rows)

    print("\nResult preview:")
    print(df.to_string(index=False))

    ans = input("\nSave to ./delegations_adjusted.csv ? [y/N]: ").strip().lower()
    if ans == "y":
        out_path = Path("./delegations_adjusted.csv")
        df.to_csv(out_path, index=False)
        print(f"Saved: {out_path.resolve()}")
    else:
        print("Not saved.")


if __name__ == "__main__":
    main()
