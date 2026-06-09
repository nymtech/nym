#!/usr/bin/env python3
"""
Unbond all nodes listed in nodes.csv.

Usage:
  python3 unbond_all.py nodes.csv [options]

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
    parser = argparse.ArgumentParser(description="Unbond all Nym nodes listed in CSV")
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


def run(cmd, dry_run: bool):
    redacted = [str(c) for c in cmd]
    if "--mnemonic" in redacted:
        i = redacted.index("--mnemonic")
        if i + 1 < len(redacted):
            redacted[i + 1] = "***REDACTED***"
    print(f"  {D}$ {' '.join(redacted)}{NC}")
    if dry_run:
        return
    subprocess.run(cmd, check=True)


def main():
    args = parse_args()
    nym_cli = resolve_nym_cli(args)

    print(f"\n  {D}nym-cli: {nym_cli}{NC}\n")

    with open(args.csv_file, newline="", encoding="utf-8") as f:
        reader = csv.DictReader(f)
        required = {"hostname", "mnemonic"}
        missing = required - set(reader.fieldnames or [])
        if missing:
            print(f"  {R}✗{NC} Missing required CSV columns: {', '.join(sorted(missing))}")
            sys.exit(1)
        nodes = list(reader)

    print(f"{W}{'═'*60}{NC}")
    dry_label = f"  {Y}[DRY RUN]{NC}" if args.dry_run else ""
    print(f"  {W}Unbonding {len(nodes)} node(s){NC}{dry_label}")
    print(f"{W}{'═'*60}{NC}\n")

     results = []
     for i, row in enumerate(nodes, 1):

        hostname = (row.get("hostname") or f"<row {i}>").strip()
        mnemonic = (row.get("mnemonic") or "").strip()
         print(f"\n{W}[{i}/{len(nodes)}]{NC} {C}{hostname}{NC}")
        if not mnemonic:
            print(f"  {R}✗{NC} Missing mnemonic")
            results.append((hostname, False))
            continue
        try:
            run([
                nym_cli, "mixnet", "operators", "nymnode", "unbond",
                "--mnemonic", mnemonic,
                "--nyxd-url", NYXD_URL,
            ], args.dry_run)
            print(f"  {G}✓{NC} Unbonded successfully")
            results.append((hostname, True))
        except subprocess.CalledProcessError as e:
            print(f"  {R}✗{NC} Failed with exit code {e.returncode}")
            results.append((hostname, False))

    print(f"\n{W}{'═'*60}{NC}")
    print(f"  {W}SUMMARY{NC}")
    print(f"{W}{'═'*60}{NC}")
    for hostname, success in results:
        status = f"{G}✓ OK    {NC}" if success else f"{R}✗ FAILED{NC}"
        print(f"  {status}  {C}{hostname}{NC}")
    succeeded = sum(1 for _, s in results if s)
    print(f"{W}{'─'*60}{NC}")
    print(f"  Total: {G}{succeeded} succeeded{NC}  {R}{len(results) - succeeded} failed{NC}")
    print(f"{W}{'═'*60}{NC}\n")


if __name__ == "__main__":
    main()