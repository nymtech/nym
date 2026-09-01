#!/usr/bin/env python3
"""
Top up each nym-node's on-chain gas account to a target balance.

Why: a node whose cosmos account holds less than 1 NYM is penalised 20% on its
config score. The node account pays gas for the node's automated on-chain calls
(ticket/reward fetching); it is DISTINCT from the operator's bonding account.

What it does, per node in the CSV:
  1. reads the node's on-chain account address from
     http://<ip>:8080/api/v2/auxiliary-details  (field: "address")
     -- no mnemonic, no SSH, read-only.
  2. reads that account's balance via `nym-cli account balance <address>`.
  3. if balance < target, queues a top-up of (target - balance).
  4. sends all queued top-ups in one batch via `nym-cli account send-multiple`,
     funded by ONE master mnemonic you provide (--mnemonic / MNEMONIC env).

The updated CSV (with node_account_address and node_account_balance filled in)
is written back so the file stays a single source of truth.

ALL AMOUNTS IN THE CSV AND FLAGS ARE IN NYM. nym-cli speaks unym internally
(1 NYM = 1_000_000 unym); conversion happens here.

Usage:
  python3 topup_nodes.py nodes.csv --mnemonic "word1 word2 ..." [options]
  MNEMONIC="word1 ..." python3 topup_nodes.py nodes.csv [options]

Options:
  --node-account-amount N   Target balance in NYM for every node
                            (overrides each row's node_account_amount).
  --cli-dir PATH            Directory containing the nym-cli binary.
  --check-only              Only read addresses + balances and write them back;
                            never send anything.
  --dry-run                 Print what would happen; no network writes.
"""
import argparse
import csv
import json
import os
import subprocess
import sys
import tempfile
import urllib.request
import urllib.error
from pathlib import Path

NYXD_URL    = "https://rpc.nymtech.net"
NYM_API_URL = "https://validator.nymtech.net/api"

# Public Cosmos LCD (REST) endpoints for reading balances over plain HTTP.
# Balance reads no longer depend on the nym-cli binary being present; nym-cli is
# only needed for the actual top-up transaction (send-multiple). Tried in order.
LCD_ENDPOINTS = [
    "https://api.nymtech.net",
    "https://api.nyx.nodes.guru",
]
LCD_BALANCE_PATH = "/cosmos/bank/v1beta1/balances/{address}"

UNYM_PER_NYM = 1_000_000

# The `nym-cli account send-multiple --input` CSV is a bare two-column file with
# NO header: each row is  <recipient_address>,<amount_with_denom>
# The amount carries its denom as a suffix, e.g. "5000000unym" (or "5nym").
# We emit unym to avoid any fractional-NYM rounding on the CLI side.

AUX_DETAILS_PATH = "/api/v2/auxiliary-details"
NODE_HTTP_PORT   = 8080

# ── Colors ──
G  = "\033[0;32m"; R = "\033[0;31m"; Y = "\033[0;33m"
C  = "\033[0;36m"; W = "\033[1;37m"; D = "\033[2;37m"; NC = "\033[0m"

def ok(m):   print(f"  {G}✓{NC} {m}")
def err(m):  print(f"  {R}✗{NC} {m}")
def info(m): print(f"  {C}→{NC} {m}")
def warn(m): print(f"  {Y}!{NC} {m}")


# ── unit helpers ────────────────────────────────────────────────────────────
def nym_to_unym(nym) -> int:
    """NYM (str/float) -> integer unym."""
    return int(round(float(nym) * UNYM_PER_NYM))

def unym_to_nym(unym) -> float:
    return float(unym) / UNYM_PER_NYM


# ── redaction ───────────────────────────────────────────────────────────────
SENSITIVE_FLAGS = {"--mnemonic", "--signature"}

def redact_cmd(cmd: list) -> list:
    out, hide = [], False
    for tok in map(str, cmd):
        if hide:
            out.append("***REDACTED***"); hide = False; continue
        out.append(tok)
        if tok in SENSITIVE_FLAGS:
            hide = True
    return out


def parse_args():
    p = argparse.ArgumentParser(description="Top up nym-node gas accounts to a target balance (NYM).")
    p.add_argument("csv_file", help="Path to nodes CSV file")
    p.add_argument("--mnemonic", default=os.environ.get("MNEMONIC"),
                   help="Master funding account mnemonic (or MNEMONIC env var).")
    p.add_argument("--node-account-amount", type=float, default=None,
                   help="Target balance in NYM for every node (overrides per-row value).")
    p.add_argument("--cli-dir", type=Path, default=None,
                   help="Directory containing the nym-cli binary.")
    p.add_argument("--check-only", action="store_true",
                   help="Only read balances and write them back; send nothing.")
    p.add_argument("--dry-run", action="store_true",
                   help="Print commands without executing network writes.")
    return p.parse_args()


def resolve_nym_cli(args):
    if args.cli_dir:
        nym_cli = args.cli_dir.resolve() / "nym-cli"
    else:
        nym_cli = Path(__file__).resolve().parents[3] / "target" / "release" / "nym-cli"
    if not nym_cli.exists() and not args.dry_run:
        err(f"nym-cli not found at: {nym_cli}")
        sys.exit(1)
    return nym_cli


# ── node address from the node's own HTTP API ───────────────────────────────
def fetch_node_address(ip: str, dry_run: bool) -> str:
    url = f"http://{ip}:{NODE_HTTP_PORT}{AUX_DETAILS_PATH}"
    if dry_run:
        return "DRY_RUN_ADDRESS"
    req = urllib.request.Request(url, headers={"User-Agent": "nym-topup/1.0"})
    with urllib.request.urlopen(req, timeout=15) as resp:
        data = json.loads(resp.read().decode("utf-8"))
    addr = data.get("address")
    if not addr:
        raise ValueError(f"no 'address' field in {url} response")
    return addr


# ── balance from the public Cosmos LCD (no nym-cli needed) ──────────────────
def fetch_balance_nym(address: str, dry_run: bool) -> float:
    """Return the account's NYM balance as a float, read over HTTP from the
    chain's LCD. Tries each LCD endpoint until one answers."""
    if dry_run:
        return 0.0
    last_err = None
    for base in LCD_ENDPOINTS:
        url = base.rstrip("/") + LCD_BALANCE_PATH.format(address=address)
        try:
            req = urllib.request.Request(url, headers={"User-Agent": "nym-topup/1.0"})
            with urllib.request.urlopen(req, timeout=15) as resp:
                data = json.loads(resp.read().decode("utf-8"))
            unym = 0
            for coin in data.get("balances", []):
                if coin.get("denom") == "unym":
                    unym = int(coin.get("amount", "0"))
                    break
            return unym_to_nym(unym)
        except (urllib.error.URLError, urllib.error.HTTPError, ValueError, KeyError, TimeoutError) as e:
            last_err = e
            continue
    raise RuntimeError(f"all LCD endpoints failed for {address}: {last_err}")


def main():
    args = parse_args()

    with open(args.csv_file, newline="", encoding="utf-8") as f:
        reader = csv.DictReader(f)
        fieldnames = list(reader.fieldnames or [])
        required = {"hostname", "ip"}
        missing = required - set(fieldnames)
        if missing:
            err(f"Missing required CSV columns: {', '.join(sorted(missing))}")
            sys.exit(1)
        rows = list(reader)

    # ensure the columns we populate exist in the output
    for col in ("node_account_address", "node_account_balance", "node_account_amount"):
        if col not in fieldnames:
            fieldnames.append(col)

    print(f"{W}{'═'*70}{NC}")
    dry = f"  {Y}[DRY RUN]{NC}" if args.dry_run else ""
    mode = "Checking" if args.check_only else "Topping up"
    print(f"  {W}{mode} {len(rows)} node account(s) (amounts in NYM){NC}{dry}")
    print(f"{W}{'═'*70}{NC}\n")

    queued = []          # (row, address, deficit_nym)
    read_errors = 0

    for i, row in enumerate(rows, 1):
        hostname = (row.get("hostname") or f"<row {i}>").strip()
        ip       = (row.get("ip") or "").strip()
        print(f"{W}[{i}/{len(rows)}]{NC} {C}{hostname}{NC}  {D}({ip}){NC}")

        if not ip:
            err("missing ip"); read_errors += 1; continue

        # 1. address
        try:
            address = (row.get("node_account_address") or "").strip()
            if not address:
                address = fetch_node_address(ip, args.dry_run)
                row["node_account_address"] = address
            info(f"account: {D}{address}{NC}")
        except (urllib.error.URLError, urllib.error.HTTPError, ValueError, TimeoutError) as e:
            err(f"could not read node address from :{NODE_HTTP_PORT}{AUX_DETAILS_PATH}: {e}")
            read_errors += 1
            continue

        # 2. balance (over HTTP from the chain LCD; no nym-cli needed)
        try:
            balance = fetch_balance_nym(address, args.dry_run)
            row["node_account_balance"] = f"{balance:.6f}"
        except Exception as e:
            err(f"balance query failed: {e}")
            read_errors += 1
            continue

        # 3. target + deficit
        if args.node_account_amount is not None:
            target = args.node_account_amount
        else:
            raw_t = (row.get("node_account_amount") or "").strip()
            if not raw_t:
                warn("no target set (node_account_amount empty and no --node-account-amount); skipping top-up")
                print(f"    balance: {G}{balance:.6f} NYM{NC}")
                continue
            target = float(raw_t)

        if balance >= target:
            ok(f"balance {balance:.6f} NYM ≥ target {target:g} NYM — no top-up needed")
        else:
            deficit = round(target - balance, 6)
            warn(f"balance {balance:.6f} NYM < target {target:g} NYM — queue +{deficit:.6f} NYM")
            queued.append((row, address, deficit))
        print()

    # write back the CSV with fresh addresses + balances
    if not args.dry_run:
        with open(args.csv_file, "w", newline="", encoding="utf-8") as f:
            writer = csv.DictWriter(f, fieldnames=fieldnames)
            writer.writeheader()
            writer.writerows(rows)
        info(f"updated {args.csv_file} with addresses + balances")

    if args.check_only:
        _summary(rows, read_errors, topped=0)
        return

    if not queued:
        print(f"\n  {G}All node accounts already at or above target.{NC}")
        _summary(rows, read_errors, topped=0)
        return

    # 4. batch send-multiple — nym-cli is only needed from here on.
    if not args.mnemonic and not args.dry_run:
        err("no master mnemonic provided (--mnemonic or MNEMONIC env) — cannot top up")
        sys.exit(1)

    nym_cli = resolve_nym_cli(args)

    print(f"{W}{'─'*70}{NC}")
    print(f"  {W}Sending {len(queued)} top-up(s) via send-multiple{NC}")
    print(f"  {D}nym-cli: {nym_cli}{NC}")
    print(f"{W}{'─'*70}{NC}")

    # build the input CSV: bare "address,amount" rows (NO header).
    # send-multiple wants the denom baked into the value, e.g. "5000000unym".
    tmp = tempfile.NamedTemporaryFile("w", suffix=".csv", delete=False, newline="")
    with tmp as tf:
        w = csv.writer(tf)
        for row, address, deficit in queued:
            w.writerow([address, f"{nym_to_unym(deficit)}unym"])
    input_csv = Path(tmp.name)

    log_csv = Path(args.csv_file).with_suffix(".topup-log.csv")

    cmd = [
        str(nym_cli), "account", "send-multiple",
        "--input",  str(input_csv),
        "--output", str(log_csv),
        "--mnemonic", args.mnemonic or "MISSING",
        "--nyxd-url", NYXD_URL,
        "--memo", "nym-node gas top-up",
    ]
    print(f"  {D}$ {' '.join(redact_cmd(cmd))}{NC}")
    for row, address, deficit in queued:
        print(f"    {D}{address}  +{deficit:.6f} NYM ({nym_to_unym(deficit)}unym){NC}")

    topped = 0
    if args.dry_run:
        info("dry run — not sending")
    else:
        result = subprocess.run(cmd, text=True)
        if result.returncode != 0:
            err(f"send-multiple failed with exit code {result.returncode}")
            try: input_csv.unlink()
            except OSError: pass
            sys.exit(1)
        topped = len(queued)
        ok(f"sent {topped} top-up(s); log: {log_csv}")

    try: input_csv.unlink()
    except OSError: pass

    _summary(rows, read_errors, topped)


def _summary(rows, read_errors, topped):
    print(f"\n{W}{'═'*70}{NC}")
    print(f"  {W}SUMMARY{NC}")
    print(f"{W}{'═'*70}{NC}")
    print(f"  {W}{'HOSTNAME':<38} {'NODE ACCOUNT':<45} BALANCE (NYM){NC}")
    print(f"  {D}{'─'*100}{NC}")
    for row in rows:
        h = (row.get("hostname") or "").strip()
        a = (row.get("node_account_address") or "?").strip()
        b = (row.get("node_account_balance") or "?").strip()
        print(f"  {C}{h:<38}{NC} {D}{a:<45}{NC} {G}{b}{NC}")
    print(f"  {D}{'─'*100}{NC}")
    print(f"  Topped up: {G}{topped}{NC}   Read errors: {R}{read_errors}{NC}\n")
    if read_errors:
        sys.exit(1)


if __name__ == "__main__":
    main()