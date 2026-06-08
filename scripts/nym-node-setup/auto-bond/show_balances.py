#!/usr/bin/env python3
"""
Check balances for all accounts in nodes.csv.
Usage: python3 show_balances.py nodes.csv [--dry-run]
"""
import csv
import subprocess
import sys
from pathlib import Path

REPO_ROOT  = Path(__file__).resolve().parents[3]
NYM_CLI    = REPO_ROOT / "target" / "release" / "nym-cli"
NYXD_URL   = "https://rpc.nymtech.net"
DRY_RUN    = "--dry-run" in sys.argv

def get_balance(account: str) -> str:
    if DRY_RUN:
        return "DRY_RUN_BALANCE"
    result = subprocess.run(
        [NYM_CLI, "account", "balance", account, "--nyxd-url", NYXD_URL],
        capture_output=True, text=True, check=True
    )
    return result.stdout.strip()

def main():
    csv_file = next((a for a in sys.argv[1:] if not a.startswith("--")), None)
    if not csv_file:
        print("Usage: check_balances.py nodes.csv [--dry-run]")
        sys.exit(1)

    with open(csv_file, newline="", encoding="utf-8") as f:
        reader = csv.DictReader(f)
        required = {"hostname", "account"}
        missing = required - set(reader.fieldnames or [])
        if missing:
            print(f"Missing required CSV columns: {', '.join(sorted(missing))}")
            sys.exit(1)
        nodes = list(reader)

    print(f"\n{'='*60}")
    print(f"  Checking {len(nodes)} account(s){'  [DRY RUN]' if DRY_RUN else ''}")
    print(f"{'='*60}\n")
    print(f"  {'HOSTNAME':<40} {'ACCOUNT':<45} BALANCE")
    print(f"  {'-'*110}")

    for row in nodes:
        hostname = row["hostname"]
        account  = row["account"]
        try:
            balance = get_balance(account)
            print(f"  {hostname:<40} {account:<45} {balance}")
        except Exception as e:
            print(f"  {hostname:<40} {account:<45} ✗ ERROR: {e}")

    print()

if __name__ == "__main__":
    main()