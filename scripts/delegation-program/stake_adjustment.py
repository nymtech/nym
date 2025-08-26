#!/usr/bin/env python3
import argparse
import csv
import sys
from pathlib import Path
import requests
import pandas as pd

API_BASE = "https://api.nym.spectredao.net/api/v1/nodes"
NYM_FACTOR = 1_000_000  # 1 NYM = 1,000,000 uNYM

"""
This simple argument based program is designed primarily for Delegation program management.
The main goal is to generate a csv which can also be reused for nym-cli as input:
./nym-cli mixnet delegators delegate-multi --mnemonic "<MNEMONIC>" --input <PATH>/<FILE>.csv

The default values therefore are:
--wallet_address: Nym Team DP wallet address
--saturation: 250k NYM
--stake_cap: 90% as per DP rules
--adjustment_Step: 25k NYM as per DP rules
--max_wallet_delegation: 125k NYM as per DP rules
--denom: NYM not uNYM to make it smoother and aligned with delegate-multi command of nym-cli
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


def fetch_delegations(node_id: int) -> list[dict]:
    url = f"{API_BASE}/{node_id}/delegations"
    r = requests.get(url, timeout=20)
    r.raise_for_status()
    data = r.json()
    if not isinstance(data, list):
        raise RuntimeError(f"Unexpected API response for node {node_id}")
    return data


def suggest_wallet_delegation(
    node_id: int,
    wallet: str,
    saturation_unym: int,
    cap_pct: int,
    step_unym: int,
    max_wallet_unym: int,
    out_denom: str,
) -> dict:
    delegs = fetch_delegations(node_id)

    # current totals (in uNYM)
    total_unym = 0
    wallet_unym = 0
    for d in delegs:
        amt = int(d["amount"]["amount"])
        total_unym += amt
        if str(d.get("owner", "")).strip() == wallet:
            wallet_unym += amt

    target_unym = (saturation_unym * cap_pct) // 100

    # start from min(current_wallet, max_wallet) and back off by step until under target
    suggested_wallet_unym = min(wallet_unym, max_wallet_unym)
    suggested_total_unym = total_unym - wallet_unym + suggested_wallet_unym

    if suggested_total_unym > target_unym and step_unym > 0:
        while suggested_total_unym > target_unym and suggested_wallet_unym > 0:
            dec = min(step_unym, suggested_wallet_unym)
            suggested_wallet_unym -= dec
            suggested_total_unym -= dec

    # convert to denom
    suggested_wallet = from_unym(suggested_wallet_unym, out_denom)
    current_wallet = from_unym(wallet_unym, out_denom)
    suggested_total = from_unym(suggested_total_unym, out_denom)
    current_total = from_unym(total_unym, out_denom)
    saturation_val = from_unym(saturation_unym, out_denom)

    # saturation as integer percentages
    suggested_sat = int((suggested_total * 100) // (saturation_val or 1))
    current_sat = int((current_total * 100) // (saturation_val or 1))

    return {
        "NODE ID": node_id,
        "SUGGESTED WALLET DELEGATION": suggested_wallet,
        "CURRENT WALLET DELEGATION": current_wallet,
        "SUGGESTED TOTAL STAKE": suggested_total,
        "CURRENT TOTAL STAKE": current_total,
        "SUGGESTED SATURATION": suggested_sat,
        "CURRENT SATURATION": current_sat,
    }


def main():
    args = parse_args()
    denom = args.denom

    # convert user inputs to uNYM
    saturation_unym = to_unym(args.saturation, denom)
    step_unym = to_unym(args.adjustment_step, denom)
    max_wallet_unym = to_unym(args.max_wallet_delegation, denom)

    node_ids = read_node_ids(args.input)

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
