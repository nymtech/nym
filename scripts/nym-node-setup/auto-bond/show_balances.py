#!/usr/bin/env python3
"""
Check balances for all accounts in nodes.csv.

Usage:
  python3 show_balances.py nodes.csv [options]

Options:
  --cli-dir PATH   Directory containing the nym-cli binary
  --dry-run        Print commands without executing
"""
import argparse
import csv
import subprocess
import sys
from pathlib import Path

NYXD_URL = "https://rpc.nymtech.net"

# ── Colors ──
G  = "\033[0;32m"
R  = "\033[0;31m"
Y  = "\033[0;33m"
C  = "\033[0;36m"
W  = "\033[1;37m"
D  = "\033[2;37m"
NC = "\033[0m"


def parse_args():
    parser = argparse.ArgumentParser(description="Check balances for all accounts in CSV")
    parser.add_argument("csv_file", help="Path to nodes CSV file")
    parser.add_argument(
        "--cli-dir",
        type=Path,
        default=None,
        help="Directory containing the nym-cli binary",
    )
    parser.add_argument("--dry-run", action="store_true", help="Print commands without executing")
    return parser.parse_args()


def resolve_nym_cli(args):
    if args.cli_dir:
        nym_cli = args.cli_dir.resolve() / "nym-cli"
    else:
        nym_cli = Path(__file__).resolve().parents[3] / "target" / "release" / "nym-cli"
    if not nym_cli.exists() and not args.dry_run:
        print(f"  {R}✗{NC} nym-cli not found at: {nym_cli}")
        sys.exit(1)
    return nym_cli


def get_balance(nym_cli: Path, account: str, dry_run: bool) -> str:
    if dry_run:
        return "DRY_RUN_BALANCE"
    result = subprocess.run(
        [nym_cli, "account", "balance", account, "--nyxd-url", NYXD_URL],
        capture_output=True, text=True, check=True
    )
    return result.stdout.strip()


def main():
    args = parse_args()
    nym_cli = resolve_nym_cli(args)

    print(f"\n  {D}nym-cli: {nym_cli}{NC}\n")

    with open(args.csv_file, newline="", encoding="utf-8") as f:
        reader = csv.DictReader(f)
        required = {"hostname", "account"}
        missing = required - set(reader.fieldnames or [])
        if missing:
            print(f"  {R}✗{NC} Missing required CSV columns: {', '.join(sorted(missing))}")
            sys.exit(1)
        nodes = list(reader)

    print(f"{W}{'═'*60}{NC}")
    dry_label = f"  {Y}[DRY RUN]{NC}" if args.dry_run else ""
    print(f"  {W}Checking {len(nodes)} account(s){NC}{dry_label}")
    print(f"{W}{'═'*60}{NC}\n")

    print(f"  {W}{'HOSTNAME':<40} {'ACCOUNT':<45} BALANCE{NC}")
    print(f"  {D}{'─'*110}{NC}")

    errors    = 0
    total_nym = 0.0

    for row in nodes:
        hostname = row["hostname"]
        account  = row["account"]
        try:
            balance = get_balance(nym_cli, account, args.dry_run)
            print(f"  {C}{hostname:<40}{NC} {D}{account:<45}{NC} {G}{balance}{NC}")
            parts = balance.split()
            if parts:
                try:
                    total_nym += float(parts[0])
                except ValueError:
                    pass
        except Exception as e:
            print(f"  {C}{hostname:<40}{NC} {D}{account:<45}{NC} {R}✗ ERROR: {e}{NC}")
            errors += 1

    print(f"  {D}{'─'*110}{NC}")
    print(f"  {W}Total balance: {G}{total_nym:,.6f} nym{NC}")
    print(f"  {W}Accounts: {G}{len(nodes) - errors} OK{NC}  {R}{errors} errors{NC}\n")
    if errors:
        sys.exit(1)

if __name__ == "__main__":
    main()