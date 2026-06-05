#!/usr/bin/env python3
"""
Automated Nym node bonding from CSV.
Usage: python3 bond_all.py nodes.csv [--dry-run]
"""
import csv
import json
import subprocess
import sys
import re
from pathlib import Path


NYXD_URL     = "https://rpc.nymtech.net"   # adjust as needed
NYM_API_URL  = "https://validator.nymtech.net/api"
REPO_ROOT = Path(__file__).resolve().parents[3]  # scripts/nym-node-setup/autobond → up 3
NYM_CLI   = REPO_ROOT / "target" / "release" / "nym-cli"
ANSIBLE_PB = REPO_ROOT / "ansible" / "auto-bond.yml"


DRY_RUN = "--dry-run" in sys.argv

def run(cmd: list, capture=True) -> subprocess.CompletedProcess:
    print(f"  $ {' '.join(str(c) for c in cmd)}")
    if DRY_RUN:
        return subprocess.CompletedProcess(cmd, 0, stdout='{"dry_run": true}', stderr="")
    return subprocess.run(cmd, capture_output=capture, text=True, check=True)

def generate_payload(row: dict) -> str:
    """Run create-node-bonding-sign-payload, return the base58 payload string."""
    result = run([
        NYM_CLI, "mixnet", "operators", "nymnode",
        "create-node-bonding-sign-payload",
        "--host",                     row["hostname"],
        "--identity-key",             row["identity_key"],
        "--amount",                   row["amount"],
        "--mnemonic",                 row["mnemonic"],
        "--interval-operating-cost",  row["operator_cost"],
        "--nyxd-url",                 NYXD_URL,
        "--nym-api-url",              NYM_API_URL,
        "-o", "json",
    ])
    if DRY_RUN:
        return "DRY_RUN_PAYLOAD"
    data = json.loads(result.stdout)
    # payload is typically under a key like "payload" or "sign_payload"
    return data.get("payload") or data.get("sign_payload") or list(data.values())[0]

def ansible_sign(ip: str, payload: str) -> str:
    """Run ansible against one host, return the signature string."""
    result = run([
        "ansible-playbook", ANSIBLE_PB,
        "--limit",    ip,
        "--tags",     "bonding",
        "--extra-vars", f"contract_msg={payload}",
    ])
    if DRY_RUN:
        return "DRY_RUN_SIGNATURE"
    # Parse signature out of ansible stdout
    # sign output is JSON: {"signature": "..."}
    match = re.search(r'"signature"\s*:\s*"([^"]+)"', result.stdout)
    if not match:
        raise ValueError(f"Could not find signature in ansible output:\n{result.stdout}")
    return match.group(1)

def bond_node(row: dict, signature: str):
    """Run nym-cli bond with the obtained signature."""
    run([
        NYM_CLI, "mixnet", "operators", "nymnode", "bond",
        "--host",                     row["hostname"],
        "--identity-key",             row["identity_key"],
        "--amount",                   row["amount"],
        "--mnemonic",                 row["mnemonic"],
        "--signature",                signature,
        "--interval-operating-cost",  row["operator_cost"],
        "--nyxd-url",                 NYXD_URL,
        "--nym-api-url",              NYM_API_URL,
        "--force",
    ], capture=False)

def main():
    csv_file = next((a for a in sys.argv[1:] if not a.startswith("--")), None)
    if not csv_file:
        print("Usage: bond_all.py nodes.csv [--dry-run]")
        sys.exit(1)

    with open(csv_file) as f:
        nodes = list(csv.DictReader(f))

    print(f"\n{'='*60}")
    print(f"  Bonding {len(nodes)} node(s){'  [DRY RUN]' if DRY_RUN else ''}")
    print(f"{'='*60}\n")

    results = []
    for i, row in enumerate(nodes, 1):
        hostname = row["hostname"]
        ip       = row["ip"]
        print(f"\n[{i}/{len(nodes)}] {hostname} ({ip})")
        try:
            print("  → Generating bonding payload…")
            payload = generate_payload(row)

            print("  → Signing on remote node via Ansible…")
            signature = ansible_sign(ip, payload)

            print("  → Submitting bond transaction…")
            bond_node(row, signature)

            results.append((hostname, "✓ OK"))
            print(f"  ✓ Bonded successfully")

        except Exception as e:
            results.append((hostname, f"✗ FAILED: {e}"))
            print(f"  ✗ Error: {e}")
            print("  Continuing with next node…")

    # Summary
    print(f"\n{'='*60}  SUMMARY  {'='*60}")
    for hostname, status in results:
        print(f"  {status:<40} {hostname}")
    print(f"{'='*120}\n")

if __name__ == "__main__":
    main()