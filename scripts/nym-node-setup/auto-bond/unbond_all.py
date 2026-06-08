#!/usr/bin/env python3
"""
Unbond all nodes listed in nodes.csv.
Usage: python3 unbond_all.py nodes.csv [--dry-run]
"""
import csv
import subprocess
import sys
from pathlib import Path

REPO_ROOT  = Path(__file__).resolve().parents[3]
NYM_CLI    = REPO_ROOT / "target" / "release" / "nym-cli"
NYXD_URL   = "https://rpc.nymtech.net"
DRY_RUN    = "--dry-run" in sys.argv

def run(cmd):
    redacted = [str(c) for c in cmd]
    if "--mnemonic" in redacted:
        i = redacted.index("--mnemonic")
        if i + 1 < len(redacted):
            redacted[i + 1] = "***REDACTED***"
    print(f"  $ {' '.join(redacted)}")
    if DRY_RUN:
        return
    subprocess.run(cmd, check=True)

def main():
    csv_file = next((a for a in sys.argv[1:] if not a.startswith("--")), None)
    if not csv_file:
        print("Usage: unbond_all.py nodes.csv [--dry-run]")
        sys.exit(1)

    with open(csv_file, newline="", encoding="utf-8") as f:
        reader = csv.DictReader(f)
        required = {"hostname", "mnemonic"}
        missing = required - set(reader.fieldnames or [])
        if missing:
            print(f"Missing required CSV columns: {', '.join(sorted(missing))}")
            sys.exit(1)
        nodes = list(reader)

    print(f"\n{'='*60}")
    print(f"  Unbonding {len(nodes)} node(s){'  [DRY RUN]' if DRY_RUN else ''}")
    print(f"{'='*60}\n")

    results = []
    for i, row in enumerate(nodes, 1):
        hostname = row["hostname"]
        print(f"\n[{i}/{len(nodes)}] {hostname}")
        try:
            run([
                NYM_CLI, "mixnet", "operators", "nymnode", "unbond",
                "--mnemonic", row["mnemonic"],
                "--nyxd-url", NYXD_URL,
            ])
            results.append((hostname, "✓ OK"))
        except subprocess.CalledProcessError as e:
            results.append((hostname, f"✗ FAILED: exit code {e.returncode}"))
            print(f"  ✗ Error: command failed with exit code {e.returncode}")

    print(f"\n{'='*60}  SUMMARY  {'='*60}")
    for hostname, status in results:
        print(f"  {status:<40} {hostname}")
    print()

if __name__ == "__main__":
    main()