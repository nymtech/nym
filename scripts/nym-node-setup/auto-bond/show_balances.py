#!/usr/bin/env python3
"""
Show balances for all accounts in nodes.csv — both the operator's BONDING
account and the node's on-chain GAS account, side by side.

These are two different accounts:
  * bonding_account_address — the operator's local wallet that owns the bond.
  * node_account_address     — the node's own on-chain account (pays gas for the
                               node's automated calls). Read live from the node's
                               HTTP API when not already in the CSV. A node with
                               < 1 NYM here is penalised 20% on its config score.

All balances are shown in NYM.

Usage:
  python3 show_balances.py nodes.csv [options]

Options:
  --dry-run        Print commands without executing.
"""
import argparse
import csv
import json
import sys
import urllib.request
import urllib.error
from pathlib import Path

NYXD_URL = "https://rpc.nymtech.net"

UNYM_PER_NYM     = 1_000_000
AUX_DETAILS_PATH = "/api/v2/auxiliary-details"
NODE_HTTP_PORT   = 8080

# Public Cosmos LCD (REST) endpoints — balances are read over plain HTTP from
# the chain, so this tool needs no nym-cli binary at all. Tried in order.
LCD_ENDPOINTS = [
    "https://api.nymtech.net",
    "https://api.nyx.nodes.guru",
]
LCD_BALANCE_PATH = "/cosmos/bank/v1beta1/balances/{address}"

G  = "\033[0;32m"
R  = "\033[0;31m"
Y  = "\033[0;33m"
C  = "\033[0;36m"
W  = "\033[1;37m"
D  = "\033[2;37m"
NC = "\033[0m"


def parse_args():
    p = argparse.ArgumentParser(description="Show bonding + node balances from CSV (NYM)")
    p.add_argument("csv_file", help="Path to nodes CSV file")
    p.add_argument("--dry-run", action="store_true", help="Print commands without executing")
    return p.parse_args()




DRY_RUN = object()   # sentinel: balance not queried because --dry-run is set


def balance_nym(address: str, dry_run: bool):
    """Return NYM balance as float, None on error, or the DRY_RUN sentinel when
    --dry-run is set (so callers can exclude it from totals/warnings rather than
    treating it as a real 0.0 balance)."""
    if dry_run:
        return DRY_RUN
    if not address:
        return None
    for base in LCD_ENDPOINTS:
        url = base.rstrip("/") + LCD_BALANCE_PATH.format(address=address)
        try:
            req = urllib.request.Request(url, headers={"User-Agent": "nym-balances/1.0"})
            with urllib.request.urlopen(req, timeout=15) as resp:
                data = json.loads(resp.read().decode("utf-8"))
            unym = 0
            for coin in data.get("balances", []):
                if coin.get("denom") == "unym":
                    unym = int(coin.get("amount", "0"))
                    break
            return unym / UNYM_PER_NYM
        except (urllib.error.URLError, urllib.error.HTTPError, ValueError, KeyError, TimeoutError):
            continue
    return None


def node_address(row: dict, ip: str, dry_run: bool):
    """Node account address from CSV, else live from the node HTTP API."""
    addr = (row.get("node_account_address") or "").strip()
    if addr:
        return addr
    if dry_run or not ip:
        return ""
    try:
        url = f"http://{ip}:{NODE_HTTP_PORT}{AUX_DETAILS_PATH}"
        req = urllib.request.Request(url, headers={"User-Agent": "nym-balances/1.0"})
        with urllib.request.urlopen(req, timeout=15) as resp:
            data = json.loads(resp.read().decode("utf-8"))
        return data.get("address", "") or ""
    except (urllib.error.URLError, urllib.error.HTTPError, ValueError, TimeoutError):
        return ""


def fmt(val):
    if val is DRY_RUN:
        return f"{D}—{NC}"
    if val is None:
        return f"{R}ERR{NC}"
    return f"{G}{val:.6f}{NC}"


def main():
    args = parse_args()

    with open(args.csv_file, newline="", encoding="utf-8") as f:
        reader = csv.DictReader(f)
        required = {"hostname"}
        missing = required - set(reader.fieldnames or [])
        if missing:
            print(f"  {R}✗{NC} Missing required CSV columns: {', '.join(sorted(missing))}")
            sys.exit(1)
        rows = list(reader)

    print(f"{W}{'═'*60}{NC}")
    dry = f"  {Y}[DRY RUN]{NC}" if args.dry_run else ""
    print(f"  {W}Checking {len(rows)} node(s) — bonding + node accounts (NYM){NC}{dry}")
    print(f"{W}{'═'*60}{NC}\n")

    print(f"  {W}{'HOSTNAME':<34} {'BONDING':>16} {'NODE':>16}{NC}")
    print(f"  {D}{'─'*70}{NC}")

    errors = 0
    total_bond = 0.0
    total_node = 0.0
    low_node   = []   # nodes under 1 NYM -> config-score penalty risk

    for row in rows:
        hostname = (row.get("hostname") or "").strip()
        ip       = (row.get("ip") or "").strip()

        bond_addr = (row.get("bonding_account_address") or "").strip()
        node_addr = node_address(row, ip, args.dry_run)

        bond_bal = balance_nym(bond_addr, args.dry_run)
        node_bal = balance_nym(node_addr, args.dry_run)

        # a DRY_RUN sentinel means "not queried" — never counts as an error,
        # a total, or a low balance. Only real errors (None) and real numbers do.
        if bond_bal is None or node_bal is None:
            errors += 1
        if isinstance(bond_bal, (int, float)):
            total_bond += bond_bal
        if isinstance(node_bal, (int, float)):
            total_node += node_bal
            if node_bal < 1.0:
                low_node.append((hostname, node_bal))

        print(f"  {C}{hostname:<34}{NC} {fmt(bond_bal):>25} {fmt(node_bal):>25}")

    print(f"  {D}{'─'*70}{NC}")
    if args.dry_run:
        print(f"  {D}(dry run — balances not queried){NC}")
    else:
        print(f"  {W}Totals:{NC}  bonding {G}{total_bond:,.6f} NYM{NC}   node {G}{total_node:,.6f} NYM{NC}")
        print(f"  {W}Nodes: {G}{len(rows) - errors} OK{NC}  {R}{errors} errors{NC}")

    if low_node:
        print(f"\n  {Y}{W}⚠ {len(low_node)} node account(s) below 1 NYM (20% config-score penalty risk):{NC}")
        for h, b in low_node:
            print(f"    {Y}!{NC} {C}{h}{NC}  {Y}{b:.6f} NYM{NC}")
        print(f"  {D}run nodes_account_topup.py to fund them.{NC}")

    print()
    if errors:
        sys.exit(1)


if __name__ == "__main__":
    main()