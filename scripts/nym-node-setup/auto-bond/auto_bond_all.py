#!/usr/bin/env python3
"""
Automated Nym node bonding from CSV.

Usage:
  python3 auto_bond_all.py nodes.csv [options]

Options:
  --ansible-repo PATH   Path to ansible playbooks directory (contains auto-bond.yml and inventory/)
  --cli-dir PATH        Path to directory containing nym-cli binary
  --dry-run             Print commands without executing

CSV format:
  inventory_node_id,hostname,ip,account,mnemonic,identity_key,amount,operator_cost
"""
import argparse
import csv
import json
import subprocess
import sys
import re
from pathlib import Path

NYXD_URL    = "https://rpc.nymtech.net"
NYM_API_URL = "https://validator.nymtech.net/api"

# ── Colors ──
G  = "\033[0;32m"   # green
R  = "\033[0;31m"   # red
Y  = "\033[0;33m"   # yellow
C  = "\033[0;36m"   # cyan
W  = "\033[1;37m"   # bold white
D  = "\033[2;37m"   # dim
NC = "\033[0m"      # reset

SENSITIVE_FLAGS = {"--mnemonic", "--signature"}


def ok(msg):   print(f"  {G}✓{NC} {msg}")
def err(msg):  print(f"  {R}✗{NC} {msg}")
def info(msg): print(f"  {C}→{NC} {msg}")
def dim(msg):  print(f"  {D}{msg}{NC}")


def redact_cmd(cmd: list) -> list:
    redacted = []
    hide_next = False
    for token in map(str, cmd):
        if hide_next:
            redacted.append("***REDACTED***")
            hide_next = False
            continue
        redacted.append(token)
        if token in SENSITIVE_FLAGS:
            hide_next = True
    return redacted


def parse_args():
    parser = argparse.ArgumentParser(description="Automated Nym node bonding from CSV")
    parser.add_argument("csv_file", help="Path to nodes CSV file")
    parser.add_argument(
        "--ansible-repo",
        type=Path,
        default=None,
        help="Path to ansible playbooks directory (contains auto-bond.yml and inventory/)",
    )
    parser.add_argument(
        "--cli-dir",
        type=Path,
        default=None,
        help="Directory containing the nym-cli binary",
    )
    parser.add_argument("--dry-run", action="store_true", help="Print commands without executing")
    return parser.parse_args()


def resolve_paths(args):
    script_dir   = Path(__file__).resolve().parent
    ansible_repo = args.ansible_repo.resolve() if args.ansible_repo else script_dir.parents[3]

    nym_cli    = (args.cli_dir.resolve() / "nym-cli") if args.cli_dir else (ansible_repo / "target" / "release" / "nym-cli")
    ansible_pb = ansible_repo / "auto-bond.yml"
    inventory  = ansible_repo / "inventory" / "all"

    errors = []
    if not nym_cli.exists():    errors.append(f"nym-cli not found at: {nym_cli}")
    if not ansible_pb.exists(): errors.append(f"auto-bond.yml not found at: {ansible_pb}")
    if not inventory.exists():  errors.append(f"inventory not found at: {inventory}")
    if errors and not args.dry_run:
        for e in errors:
            err(e)
        sys.exit(1)

    return nym_cli, ansible_pb, inventory


def run(cmd: list, dry_run: bool, capture=True, cwd=None) -> subprocess.CompletedProcess:
    print(f"  {D}$ {' '.join(redact_cmd(cmd))}{NC}")
    if dry_run:
        return subprocess.CompletedProcess(cmd, 0, stdout='{"dry_run": true}', stderr="")
    result = subprocess.run(cmd, capture_output=capture, text=True, cwd=cwd)
    if result.returncode != 0:
        if result.stdout: print(result.stdout)
        if result.stderr: print(f"{R}{result.stderr}{NC}")
        result.check_returncode()
    return result


def extract_ansible_recap(output: str):
    """Extract PLAY RECAP block from ansible stdout."""
    match = re.search(r"(PLAY RECAP \*+.*?)(?=\n\n|\Z)", output, re.DOTALL)
    return match.group(0).strip() if match else None


def generate_payload(row: dict, nym_cli: Path, dry_run: bool) -> str:
    result = run([
        nym_cli, "mixnet", "operators", "nymnode",
        "create-node-bonding-sign-payload",
        "--host",                    row["hostname"],
        "--identity-key",            row["identity_key"],
        "--amount",                  row["amount"],
        "--mnemonic",                row["mnemonic"],
        "--interval-operating-cost", row["operator_cost"],
        "--nyxd-url",                NYXD_URL,
        "--nym-api-url",             NYM_API_URL,
        "-o", "json",
    ], dry_run)
    if dry_run:
        return "DRY_RUN_PAYLOAD"
    data = json.loads(result.stdout)
    return data.get("payload") or data.get("sign_payload") or list(data.values())[0]


def ansible_sign(node_id: str, payload: str, ansible_pb: Path, inventory: Path, dry_run: bool):
    """Returns (signature, recap_block)."""
    result = run([
        "ansible-playbook", ansible_pb,
        "-i",           inventory,
        "--limit",      node_id,
        "--tags",       "bonding",
        "--extra-vars", f"contract_msg={payload}",
    ], dry_run, cwd=ansible_pb.parent)

    if dry_run:
        return "DRY_RUN_SIGNATURE", "DRY_RUN_RECAP"

    recap = extract_ansible_recap(result.stdout)
    match = re.search(r'ENCODED_SIGNATURE=([1-9A-HJ-NP-Za-km-z]+)', result.stdout)
    if not match:
        raise ValueError(f"Could not find ENCODED_SIGNATURE in ansible output:\n{result.stdout}")

    return match.group(1), recap


def bond_node(row: dict, signature: str, nym_cli: Path, dry_run: bool):
    cmd = [
        nym_cli, "mixnet", "operators", "nymnode", "bond",
        "--host",                    row["hostname"],
        "--identity-key",            row["identity_key"],
        "--amount",                  row["amount"],
        "--mnemonic",                row["mnemonic"],
        "--signature",               signature,
        "--interval-operating-cost", row["operator_cost"],
        "--nyxd-url",                NYXD_URL,
        "--nym-api-url",             NYM_API_URL,
        "--force",
    ]
    print(f"  {D}$ {' '.join(redact_cmd(cmd))}{NC}")
    if dry_run:
        return
    result = subprocess.run(cmd, text=True)
    if result.returncode != 0:
        raise RuntimeError(f"bond command failed with exit code {result.returncode}")


def main():
    args = parse_args()
    nym_cli, ansible_pb, inventory = resolve_paths(args)

    print(f"\n  {D}nym-cli  : {nym_cli}{NC}")
    print(f"  {D}playbook : {ansible_pb}{NC}")
    print(f"  {D}inventory: {inventory}{NC}\n")

    with open(args.csv_file) as f:
        nodes = list(csv.DictReader(f))

    print(f"{W}{'═'*70}{NC}")
    dry_label = f"  {Y}[DRY RUN]{NC}" if args.dry_run else ""
    print(f"  {W}Bonding {len(nodes)} node(s){NC}{dry_label}")
    print(f"{W}{'═'*70}{NC}\n")

    results  = []
    failures = []

    for i, row in enumerate(nodes, 1):
        hostname = row["hostname"]
        node_id  = row["inventory_node_id"]

        print(f"\n{W}[{i}/{len(nodes)}]{NC} {C}{hostname}{NC}  {D}({node_id}){NC}")
        print(f"  {D}{'─'*60}{NC}")

        recap = None
        try:
            info("Generating bonding payload…")
            payload = generate_payload(row, nym_cli, args.dry_run)

            info("Signing on remote node via Ansible…")
            signature, recap = ansible_sign(node_id, payload, ansible_pb, inventory, args.dry_run)

            if recap:
                print(f"\n  {D}{recap}{NC}\n")

            info("Submitting bond transaction…")
            bond_node(row, signature, nym_cli, args.dry_run)

            ok("Bonded successfully")
            results.append((hostname, True, recap, None))

        except Exception as e:
            error_msg = str(e)
            err(f"FAILED: {error_msg}")
            results.append((hostname, False, recap, error_msg))
            failures.append((hostname, error_msg))
            print(f"  {Y}↳ Continuing with next node…{NC}")

    # ── Final summary ──
    print(f"\n{W}{'═'*70}{NC}")
    print(f"  {W}SUMMARY{NC}")
    print(f"{W}{'═'*70}{NC}")

    for hostname, success, recap, error_msg in results:
        status = f"{G}✓ OK    {NC}" if success else f"{R}✗ FAILED{NC}"
        print(f"  {status}  {C}{hostname}{NC}")

    print(f"{W}{'─'*70}{NC}")
    succeeded = len(results) - len(failures)
    print(f"  Total: {G}{succeeded} succeeded{NC}  {R}{len(failures)} failed{NC}")

    if failures:
        print(f"\n  {R}{W}FAILED NODES:{NC}")
        for hostname, reason in failures:
            print(f"    {R}✗{NC} {C}{hostname}{NC}")
            print(f"      {D}{reason}{NC}")

    print(f"{W}{'═'*70}{NC}\n")

    if failures:
        sys.exit(1)


if __name__ == "__main__":
    main()