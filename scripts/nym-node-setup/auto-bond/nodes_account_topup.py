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
from decimal import Decimal
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

# The `nym-cli account send-multiple --input` CSV is a bare file with NO header,
# THREE columns per row:  <recipient_address>,<amount>,<denom>
# e.g.  n1...,5000000,unym
# Amount is the integer micro-denomination; denom is its own column ("unym").

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
    """NYM (str/float/Decimal) -> integer unym.

    Uses Decimal (not float) so decimal amounts (including fractional deficits
    like target - balance) are preserved exactly. Rejects amounts finer than
    1 unym (more than 6 decimal places) rather than silently rounding them away.
    """
    scaled = Decimal(str(nym)) * UNYM_PER_NYM
    if scaled != scaled.to_integral_value():
        raise ValueError(f"NYM amount '{nym}' has more than 6 decimal places (sub-unym precision)")
    return int(scaled.to_integral_value())

def unym_to_nym(unym) -> Decimal:
    """Integer unym -> NYM as an exact Decimal (never float)."""
    return Decimal(int(unym)) / UNYM_PER_NYM


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
    p.add_argument("-y", "--assume-yes", action="store_true",
                   help="Auto-confirm the nym-cli transfer prompt (non-interactive). "
                        "Without this, you'll be asked to confirm the transfer table.")
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

        # 3. target + deficit (all Decimal — exact, no float noise)
        if args.node_account_amount is not None:
            target = Decimal(str(args.node_account_amount))
        else:
            raw_t = (row.get("node_account_amount") or "").strip()
            if not raw_t:
                warn("no target set (node_account_amount empty and no --node-account-amount); skipping top-up")
                print(f"    balance: {G}{balance:.6f} NYM{NC}")
                continue
            target = Decimal(raw_t)

        if balance >= target:
            ok(f"balance {balance:.6f} NYM ≥ target {target:g} NYM — no top-up needed")
        else:
            deficit = target - balance
            warn(f"balance {balance:.6f} NYM < target {target:g} NYM — queue +{deficit:.6f} NYM")
            queued.append((row, address, deficit, target))
        print()

    # write back the CSV with fresh addresses + (pre-transfer) balances
    def _write_csv():
        if args.dry_run:
            return
        with open(args.csv_file, "w", newline="", encoding="utf-8") as f:
            writer = csv.DictWriter(f, fieldnames=fieldnames)
            writer.writeheader()
            writer.writerows(rows)

    _write_csv()
    if not args.dry_run:
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

    # build the input CSV: bare rows (NO header), three columns:
    #   <address>,<amount>,<denom>   e.g.  n1...,5000000,unym
    tmp = tempfile.NamedTemporaryFile("w", suffix=".csv", delete=False, newline="")
    with tmp as tf:
        w = csv.writer(tf)
        for row, address, deficit, target in queued:
            w.writerow([address, nym_to_unym(deficit), "unym"])
    input_csv = Path(tmp.name)

    log_csv = Path(args.csv_file).with_suffix(".topup-log.csv")

    # Remove any stale log from a previous run so that, after this run, the
    # log's presence/size genuinely reflects whether THIS send produced output.
    if not args.dry_run:
        try: log_csv.unlink()
        except OSError: pass

    cmd = [
        str(nym_cli), "account", "send-multiple",
        "--input",  str(input_csv),
        "--output", str(log_csv),
        "--mnemonic", args.mnemonic or "MISSING",
        "--nyxd-url", NYXD_URL,
        "--memo", "nym-node gas top-up",
    ]
    print(f"  {D}$ {' '.join(redact_cmd(cmd))}{NC}")
    for row, address, deficit, target in queued:
        print(f"    {D}{address}  +{deficit:.6f} NYM ({nym_to_unym(deficit)} unym){NC}")

    topped = 0
    if args.dry_run:
        info("dry run — not sending")
    else:
        if not args.assume_yes:
            # nym-cli will show the transfer table and prompt for confirmation;
            # answer it interactively (output is NOT captured so the prompt shows).
            result = subprocess.run(cmd, text=True)
            combined = ""  # we didn't capture; rely on exit code + log below
        else:
            # non-interactive: auto-confirm by feeding "y" to the prompt.
            # Stream output live AND collect it so we can scan for error markers
            # (nym-cli has logged fatal errors while still exiting 0).
            proc = subprocess.Popen(
                cmd, text=True, stdin=subprocess.PIPE,
                stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
            )
            captured = []
            try:
                proc.stdin.write("y\n")
                proc.stdin.flush()
                proc.stdin.close()
            except (BrokenPipeError, OSError):
                pass
            for line in proc.stdout:
                print(line, end="")
                captured.append(line)
            proc.wait()
            result = subprocess.CompletedProcess(cmd, proc.returncode)
            combined = "".join(captured)

        failed_markers = ("Failed to read input file", "ERROR", "error trying to",
                          "does not have enough columns", "insufficient funds")
        looks_failed = any(m in combined for m in failed_markers)

        # success requires: exit 0, no failure markers, and an output log that
        # actually recorded the sends.
        log_ok = log_csv.exists() and log_csv.stat().st_size > 0

        if result.returncode != 0 or looks_failed or not log_ok:
            err("send-multiple failed — no top-ups were sent")
            if result.returncode != 0:
                err(f"exit code: {result.returncode}")
            if looks_failed:
                err("error reported in nym-cli output (see above)")
            if not log_ok:
                err(f"output log missing or empty: {log_csv}")
            try: input_csv.unlink()
            except OSError: pass
            _summary(rows, read_errors, topped=0)
            sys.exit(1)

        topped = len(queued)
        ok(f"sent {topped} top-up(s); log: {log_csv}")

        # reflect the post-transfer balances: a successful top-up brings each
        # queued node up to its target. Update the rows and rewrite the CSV so
        # node_account_balance and the summary show the new, not the old, value.
        for row, address, deficit, target in queued:
            row["node_account_balance"] = f"{target:.6f}"
        _write_csv()
        info(f"updated {args.csv_file} with post-transfer balances")

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