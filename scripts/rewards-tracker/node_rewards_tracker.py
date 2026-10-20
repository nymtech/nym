#!/usr/bin/env python3
# -*- coding: utf-8 -*-

"""
Fetch operator rewards for the Nyx account addresses listed in
data/wallet-addresses.csv.

Output:
    1. Per-entity tables printed in the terminal, plus a grand summary
    2. Full sheet written to data/node-balances.csv
    3. Historical snapshots in data/data.yaml - do not edit by hand; entries
       older than the retention window are pruned automatically

data/wallet-addresses.csv columns:

    address   your Nyx account address (required)
    tag       entity name, e.g. "mysquad" or "personal", to group the output
    members   how many people share this entity's rewards (optional, default 1)

`members` drives the "your share" figures: a tag with 3 members reports both the
full node rewards and your 1/3 of them. Leaving the column out keeps the old
behaviour exactly, since every entity then counts as a single member.

PRIVACY: by default every API call fetches the whole network in one request and
filters locally, so an API operator cannot tell which wallets you care about.
Passing --fetch-wallet-balances makes one request per wallet, which lets whoever
runs that API correlate all your wallets into a single identity. It is off by
default for that reason.

Run it from anywhere: paths are resolved relative to this file, not the shell's
working directory.
"""

import argparse
import csv
import os
import sys
import time
import yaml
import requests
from collections import defaultdict
from pathlib import Path
from typing import Any, Dict, List, Tuple, Optional
from tabulate import tabulate
from colorama import init as colorama_init, Fore, Style

colorama_init()

# Resolve against this file so the script works from any working directory.
SCRIPT_DIR = Path(__file__).resolve().parent
DATA_DIR = str(SCRIPT_DIR / "data")
ADDR_CSV = os.path.join(DATA_DIR, "wallet-addresses.csv")
OUT_CSV = os.path.join(DATA_DIR, "node-balances.csv")
HIST_FILE = os.path.join(DATA_DIR, "data.yaml")

ADDR_CSV_TEMPLATE = """\
address,tag,members
# Fill in one row per Nyx account address.
#   address  required
#   tag      optional entity name, groups the output
#   members  optional, how many people share this entity (default 1)
# Delete these comment lines once you have added your own rows.
"""

SPECTRE_NODES_URL = "https://api.nym.spectredao.net/api/v1/nodes"
VALIDATOR_BONDED_URL = "https://validator.nymtech.net/api/v1/nym-nodes/bonded"
VALIDATOR_DESC_URL = "https://validator.nymtech.net/api/v2/nym-nodes/described"
SPECTRE_BAL_URL = "https://api.nym.spectredao.net/api/v1/balances/{address}"

# Spectre's /nodes endpoint returns a bare list with NO pagination envelope and
# caps at 200 records unless an explicit limit is supplied. It ignores page/size.
# This is high enough to cover the whole network with headroom.
SPECTRE_LIMIT = 10000

SESSION = requests.Session()
SESSION.headers.update({"User-Agent": "nym-tools/1.0"})


# When --raw is used the script must emit exactly one machine-readable number
# on stdout and nothing else, so progress logging is suppressed.
RAW_MODE = False


def log(msg: str) -> None:
    if RAW_MODE:
        return
    print(msg, flush=True)


def now_ts() -> float:
    return time.time()


def to_float(x: Any, default: float = 0.0) -> float:
    try:
        if x is None:
            return default
        if isinstance(x, (int, float)):
            return float(x)
        if isinstance(x, str):
            return float(x)
        return default
    except Exception:
        return default


def to_int(x: Any, default: int = 0) -> int:
    try:
        if x is None:
            return default
        if isinstance(x, int):
            return x
        if isinstance(x, float):
            return int(x)
        if isinstance(x, str):
            return int(float(x))
        return default
    except Exception:
        return default


# pagination helpers

def _get_json(url: str, params: Dict[str, Any], timeout: int = 60) -> Any:
    r = SESSION.get(url, params=params, timeout=timeout)
    r.raise_for_status()
    return r.json()

def _extract_items_and_total(data: Any) -> Tuple[List[Any], int]:
    """Return (items, total) from either a bare list or a {data, pagination} envelope.
    total is -1 when unknown."""
    if isinstance(data, dict) and "data" in data and isinstance(data["data"], list):
        items = data["data"]
        total = to_int((data.get("pagination") or {}).get("total"), -1)
        return items, total
    if isinstance(data, list):
        return data, -1
    return [], -1

def _fetch_all_page_pagesize(url: str, page_size: int = 1000, timeout: int = 60) -> List[Any]:
    """Paginate with page/size.

    NOTE: the validator API currently ignores page/size and returns the full set
    in one response, always reporting pagination.total == full count and
    page == 0. We therefore stop as soon as we have collected `total` records,
    and additionally guard against an API that returns the same page repeatedly
    (which would otherwise loop until the safety cap and massively duplicate rows).
    """
    out: List[Any] = []
    page = 0
    first_batch: Optional[List[Any]] = None
    while True:
        data = _get_json(url, {"page": page, "size": page_size}, timeout=timeout)
        items, total = _extract_items_and_total(data)
        if not items:
            break

        # Authoritative stop: we have everything the server says exists.
        if total >= 0 and len(out) + len(items) >= total:
            out.extend(items)
            break

        # Degenerate guard: total unknown and the server keeps returning the
        # exact same content regardless of page — bail rather than loop.
        if total < 0 and first_batch is not None and items == first_batch:
            break
        if first_batch is None:
            first_batch = items

        out.extend(items)
        if len(items) < page_size:
            break

        page += 1
        if page > 500:
            break
    return out

def _fetch_all_single(url: str, timeout: int = 60) -> List[Any]:
    data = _get_json(url, {}, timeout=timeout)
    items, _ = _extract_items_and_total(data)
    return items

def _fetch_all_any(url: str, timeout: int = 60) -> list:
    # page/size is the API's current pagination scheme and correctly honours
    # pagination.total, so it is safe even when the server returns everything at
    # once. The old limit/offset scheme is no longer supported by the API (the
    # params are silently ignored), which caused the same full page to be
    # re-fetched up to the safety cap and produced hundreds of thousands of
    # duplicate rows. It has been removed.
    got = _fetch_all_page_pagesize(url, page_size=1000, timeout=timeout)
    if isinstance(got, list) and got:
        return got
    return _fetch_all_single(url, timeout=timeout)


# load data

def ensure_addr_csv(path: str) -> bool:
    """Create a template wallet CSV on first run. Returns True if it was created.

    Written locally rather than fetched from GitHub: it is three lines, and a
    local template keeps first-run working even if the repo moves, the raw host
    rate-limits, or the machine is behind a restrictive proxy.
    """
    if os.path.exists(path):
        return False
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        f.write(ADDR_CSV_TEMPLATE)
    return True


def read_wallets_csv(path: str) -> List[Tuple[str, str, int]]:
    """Return [(address, tag, members)].

    `members` is optional and defaults to 1, so a pre-existing two-column file
    keeps working untouched. Values below 1 are clamped to 1 so a typo cannot
    divide by zero or invert the share.
    """
    if not os.path.exists(path):
        raise FileNotFoundError(f"Input CSV not found: {path}")
    rows: List[Tuple[str, str, int]] = []
    with open(path, newline="", encoding="utf-8") as f:
        reader = csv.DictReader(f)
        for row in reader:
            addr = (row.get("address") or "").strip()
            # tolerate the commented template lines
            if not addr or addr.startswith("#"):
                continue
            tag = (row.get("tag") or "").strip()
            members = to_int(row.get("members"), 1)
            if members < 1:
                members = 1
            rows.append((addr, tag, members))
    return rows

def fetch_nodes_all() -> List[Dict[str, Any]]:
    log(f"* * * Fetching nodes from {SPECTRE_NODES_URL} * * *")
    # Spectre ignores page/size and caps at 200 without an explicit limit, so we
    # request the full set directly rather than going through the generic
    # paginator (which would silently stop at the 200 default).
    data = _get_json(SPECTRE_NODES_URL, {"limit": SPECTRE_LIMIT}, timeout=90)
    nodes, _ = _extract_items_and_total(data)
    log(f"Fetched {len(nodes)} node(s)")
    return nodes

def fetch_bonded_all() -> List[Dict[str, Any]]:
    log(f"* * *Fetching bonded from {VALIDATOR_BONDED_URL} * * *")
    bonded = _fetch_all_any(VALIDATOR_BONDED_URL, timeout=90)
    log(f"Fetched {len(bonded)} bonded record(s)")
    return bonded

def fetch_described_all() -> List[Dict[str, Any]]:
    log(f"* * * Fetching described from {VALIDATOR_DESC_URL} * * *")
    described = _fetch_all_any(VALIDATOR_DESC_URL, timeout=90)
    log(f"Fetched {len(described)} described record(s)")
    return described

def fetch_balance_total_nym(address: str) -> float:
    url = SPECTRE_BAL_URL.format(address=address)
    try:
        r = SESSION.get(url, timeout=30)
        r.raise_for_status()
        js = r.json()
        amt = to_float(js.get("total", {}).get("amount"), 0.0)
        return amt / 1_000_000.0
    except Exception as e:
        log(f"{Fore.YELLOW}* * * warn: balance fetch failed for {address}: {e}{Style.RESET_ALL} * * *")
        return 0.0


# extract version

def _first_str(*vals) -> str:
    for v in vals:
        if isinstance(v, (str, int, float)):
            s = str(v).strip()
            if s:
                return s
    return ""

def extract_version_from_node(n: Dict[str, Any]) -> str:
    desc = n.get("description") or {}
    bi   = n.get("build_information") or {}
    return _first_str(
        n.get("version"),
        n.get("node_version"),
        bi.get("build_version"),
        bi.get("version"),
        (desc.get("software") or {}).get("version"),
        (desc.get("build_information") or {}).get("build_version"),
    )

def extract_version_from_desc(d: Dict[str, Any]) -> str:
    desc = d.get("description") or {}
    bi   = d.get("build_information") or {}
    return _first_str(
        d.get("version"),
        d.get("node_version"),
        bi.get("build_version"),
        bi.get("version"),
        (desc.get("software") or {}).get("version"),
        (desc.get("build_information") or {}).get("build_version"),
    )


# history storage in data/data.yaml + 30d cleanup + window helpers

def load_history(path: str) -> Dict[str, Any]:
    """Load data.yaml, repairing older/malformed shapes rather than discarding.

    Earlier versions of this script wrote entries that the window maths cannot
    read (missing `ts`, a bare number instead of a mapping, or a scalar in place
    of the per-node list). Those used to make every window report "no data"
    while appearing to have history. Anything unreadable is skipped per-entry;
    everything valid is kept, so no usable history is ever lost.
    """
    if not os.path.exists(path):
        return {}
    with open(path, "r", encoding="utf-8") as f:
        try:
            raw = yaml.safe_load(f) or {}
        except Exception:
            return {}
    if not isinstance(raw, dict):
        return {}

    cleaned: Dict[str, Any] = {}
    dropped = 0
    for key, lst in raw.items():
        if not isinstance(lst, list):
            dropped += 1
            continue
        good = []
        for e in lst:
            if not isinstance(e, dict):
                dropped += 1
                continue
            ts = to_float(e.get("ts"), 0.0)
            if ts <= 0:
                dropped += 1
                continue
            good.append({
                "ts": ts,
                "uptime": to_float(e.get("uptime"), 0.0),
                "operator_balance": to_float(e.get("operator_balance"), 0.0),
            })
        if good:
            cleaned[str(key)] = good
    if dropped:
        log(f"{Fore.YELLOW}* * * note: skipped {dropped} unreadable history "
            f"entr{'y' if dropped == 1 else 'ies'} in {os.path.basename(path)} "
            f"(older format); the rest was kept * * *{Style.RESET_ALL}")
    return cleaned

def save_history(path: str, data: Dict[str, Any]) -> None:
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        yaml.safe_dump(data, f, sort_keys=True)

def add_history_point(hist: Dict[str, Any], node_id: int, epoch_ts: float, uptime: float, op_bal: float) -> None:
    key = str(node_id)
    lst = hist.setdefault(key, [])
    lst.append({
        "ts": epoch_ts,
        "uptime": round(float(uptime), 6),
        "operator_balance": round(float(op_bal), 6),
    })

def last_snapshot(hist: Dict[str, Any], node_id: int) -> Optional[Dict[str, Any]]:
    key = str(node_id)
    if key not in hist:
        return None
    lst = hist[key]
    if not lst:
        return None
    return sorted(lst, key=lambda x: x.get("ts", 0.0))[-1]

def cleanup_history_older_than(hist: Dict[str, Any], cutoff_ts: float) -> None:
    # remove entries older than cutoff - 30 days
    for key, lst in list(hist.items()):
        new_lst = [e for e in lst if to_float(e.get("ts"), 0.0) >= cutoff_ts]
        if new_lst:
            hist[key] = new_lst
        else:
            del hist[key]

def scaled_window_change(
    hist: Dict[str, Any],
    node_id: int,
    now: float,
    window_days: float,
    current_balance: float
) -> Optional[Tuple[float, float, float, bool]]:
    """Return (profit_scaled, span_hours, hourly_scaled, projected) or None.

    `projected` is True when the stored history is shorter than the requested
    window, meaning the figure is an extrapolation from a shorter span rather
    than a measurement over the full window. Reporting that flag matters: with
    under 30 days of history a 30-day figure is just the 7-day one scaled up,
    and presenting it unqualified reads as real data when it is not.
    """
    key = str(node_id)
    if key not in hist or not hist[key]:
        return None
    cutoff_ts = now - window_days * 24 * 3600
    ordered = sorted(hist[key], key=lambda x: to_float(x.get("ts"), 0.0))

    # Prefer the most recent snapshot at/older than the cutoff (a true
    # window-length comparison). If none is that old yet, fall back to the
    # OLDEST snapshot and flag the result as projected.
    candidates = [e for e in ordered if to_float(e.get("ts"), 0.0) <= cutoff_ts]
    snap = candidates[-1] if candidates else ordered[0]
    projected = not candidates

    span_hours = max(0.0, (now - to_float(snap.get("ts"), now)) / 3600.0)
    if span_hours <= 0:
        return None
    profit_raw = current_balance - to_float(snap.get("operator_balance"), 0.0)
    target_hours = window_days * 24.0
    profit_scaled = profit_raw * (target_hours / span_hours)
    hourly_scaled = profit_scaled / target_hours
    return profit_scaled, span_hours, hourly_scaled, projected


def window_cell(win: Optional[Tuple[float, float, float, bool]], idx: int, days: int) -> str:
    """Format a window value, marking projections with ~ and the real span."""
    if not win:
        return f"no {days}d data"
    val = win[idx]
    if win[3]:
        return f"~{val:.6f} ({win[1] / 24.0:.1f}d)"
    return f"{val:.6f}"


# output coloring fns

def colorize(text: str, color_name: str) -> str:
    mapping = {
        "green": Fore.GREEN,
        "yellow": Fore.YELLOW,
        "orange": Fore.MAGENTA,
        "red": Fore.RED,
    }
    c = mapping.get(color_name, "")
    if not c:
        return text
    return f"{c}{text}{Style.RESET_ALL}"

def uptime_color_name(u: float) -> str:
    if u >= 0.95:
        return "green"
    if u >= 0.90:
        return "yellow"
    if u >= 0.80:
        return "orange"
    return "red"


# main program body

def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(
        description="Track Nym operator rewards per wallet and entity.",
    )
    p.add_argument(
        "--fetch-wallet-balances",
        action="store_true",
        help=(
            "Query each wallet's balance individually when bulk data reports "
            "nothing. PRIVACY: this makes one request per wallet from your IP, "
            "letting the API operator correlate all your wallets into one "
            "identity. Off by default."
        ),
    )
    p.add_argument(
        "--raw",
        choices=["total", "share"],
        default=None,
        help=(
            "Print a single raw number to stdout and nothing else, for piping "
            "into other programs. 'total' is the gross rewards across all "
            "entities, 'share' is the sum after the per-entity 1/N split. "
            "All tables, progress output and colour are suppressed; the CSV "
            "and history file are still written as usual."
        ),
    )
    p.add_argument(
        "--no-write",
        action="store_true",
        help=(
            "Do not touch anything in the data directory: no history snapshot, "
            "no CSV. A read-only query. Implied by --raw, since that is meant "
            "for piping a number into another program rather than recording a "
            "measurement."
        ),
    )
    p.add_argument(
        "--write",
        action="store_true",
        help=(
            "Force the normal writes even with --raw. Use this if you want a "
            "raw number AND a history snapshot recorded from the same run."
        ),
    )
    p.add_argument(
        "--data-dir",
        default=None,
        help="Override the data directory (default: ./data beside this script).",
    )
    return p.parse_args()


def main() -> None:
    global DATA_DIR, ADDR_CSV, OUT_CSV, HIST_FILE, RAW_MODE
    args = parse_args()
    RAW_MODE = args.raw is not None
    if args.data_dir:
        DATA_DIR = str(Path(args.data_dir).expanduser().resolve())
        ADDR_CSV = os.path.join(DATA_DIR, "wallet-addresses.csv")
        OUT_CSV = os.path.join(DATA_DIR, "node-balances.csv")
        HIST_FILE = os.path.join(DATA_DIR, "data.yaml")

    # --raw is a query, so it defaults to leaving the data directory alone.
    # --write opts back in; --no-write forces read-only in any mode.
    writes_enabled = not (args.no_write or (RAW_MODE and not args.write))

    if writes_enabled:
        os.makedirs(DATA_DIR, exist_ok=True)

    # In raw mode the caller is parsing stdout, so a missing wallet CSV must be
    # a hard error on stderr rather than silence: log() is suppressed, and
    # exiting 0 with no output looks like "no rewards" to the calling program.
    if not os.path.exists(ADDR_CSV):
        # Creating the template is itself a write, so a read-only run must
        # report the missing file instead: --no-write promises not to touch
        # anything in the data directory, and that has to include not creating
        # the directory or the template in the first place.
        if RAW_MODE or not writes_enabled:
            print(f"error: no wallet list at {ADDR_CSV}", file=sys.stderr)
            sys.exit(1)
        os.makedirs(DATA_DIR, exist_ok=True)
        if ensure_addr_csv(ADDR_CSV):
            log(f"{Fore.GREEN}Created {ADDR_CSV}{Style.RESET_ALL}")
            log("Add your Nyx account addresses to it, then run this again.")
            log("Columns: address, tag (optional), members (optional, default 1)")
            return

    wallets = read_wallets_csv(ADDR_CSV)
    if not wallets:
        if RAW_MODE:
            print(f"error: no wallets listed in {ADDR_CSV}", file=sys.stderr)
            sys.exit(1)
        log(f"{Fore.RED}No wallets found in {ADDR_CSV}{Style.RESET_ALL}")
        log("Add at least one row with an `address` value, then run again.")
        sys.exit(1)
    log(f"Found {len(wallets)} wallet(s) in {ADDR_CSV}")

    # preserve input order per wallet for per-tag tables
    wallet_order = {addr: idx for idx, (addr, _tag, _m) in enumerate(wallets)}
    # members is declared per wallet row; a tag takes the largest value seen for it
    members_by_tag: Dict[str, int] = {}
    for _addr, tag, m in wallets:
        members_by_tag[tag] = max(members_by_tag.get(tag, 1), m)

    nodes = fetch_nodes_all()
    bonded = fetch_bonded_all()
    described = fetch_described_all()

    # indexes
    idx_nodes_by_wallet: Dict[str, List[Dict[str, Any]]] = defaultdict(list)
    for n in nodes:
        w = (n.get("bonding_address") or "").strip()
        if w:
            idx_nodes_by_wallet[w].append(n)

    idx_desc_by_node_id: Dict[int, Dict[str, Any]] = {}
    for d in described:
        nid = to_int(d.get("node_id"), 0)
        if nid:
            idx_desc_by_node_id[nid] = d

    idx_bonded_by_owner: Dict[str, List[Dict[str, Any]]] = defaultdict(list)
    for b in bonded:
        bi = b.get("bond_information", {})
        owner = (bi.get("owner") or "").strip()
        if owner:
            idx_bonded_by_owner[owner].append(b)

    headers_csv = [
        "node_id",
        "hostname",
        "identity_key",
        "wallet",
        "uptime",
        "version",
        "operator_balance",
        "profit_difference",
        "epochs",
        "average_hour",
        "7_days",
        "7_days_average",
        "30_days",
        "30_days_average",
        "tag",
        "members",
        "your_share",
    ]

    hist = load_history(HIST_FILE)
    now = now_ts()

    out_rows: List[Dict[str, Any]] = []
    rows_by_tag: Dict[str, List[Dict[str, Any]]] = defaultdict(list)

    # Retain a grace buffer beyond the largest reporting window (30 days).
    # The window comparison needs a snapshot at/just past its cutoff; if cleanup
    # pruned everything older than exactly 30 days, no such point survives and
    # the 30-day figure could never be computed (the historical catch-22 where
    # it always printed "no 30 days data stored"). Keeping ~32 days guarantees a
    # comparison point just outside the 30-day boundary always remains.
    THIRTY_DAYS_SEC = 30 * 24 * 3600
    RETENTION_SEC = 32 * 24 * 3600
    cleanup_history_older_than(hist, now - RETENTION_SEC)  # prune before use

    for wallet_addr, tag, members in wallets:
        wallet_nodes = idx_nodes_by_wallet.get(wallet_addr, [])

        if not wallet_nodes:
            # fallback via bonded -> described + balance
            for b in idx_bonded_by_owner.get(wallet_addr, []):
                bi = b.get("bond_information", {})
                nid = to_int(bi.get("node_id"), 0)
                if nid <= 0:
                    continue
                d = idx_desc_by_node_id.get(nid, {})
                desc = d.get("description", {}) if isinstance(d, dict) else {}
                hostinfo = desc.get("host_information", {}) if isinstance(desc, dict) else {}
                hostname = hostinfo.get("hostname") or ""
                node = bi.get("node", {}) if isinstance(bi, dict) else {}
                identity_key = node.get("identity_key") or ""

                op_bal = (fetch_balance_total_nym(wallet_addr)
                          if args.fetch_wallet_balances else 0.0)
                uptime = 0.0

                # since last time change calculation
                prev = last_snapshot(hist, nid)
                prev_bal = to_float(prev.get("operator_balance"), 0.0) if prev else None
                prev_ts = to_float(prev.get("ts"), 0.0) if prev else None
                diff = 0.0
                hours = 0.0
                if prev is not None:
                    diff = op_bal - prev_bal
                    hours = max(0.0, (now - prev_ts) / 3600.0)

                # last 7 / 30 days calculation
                seven = scaled_window_change(hist, nid, now, 7.0, op_bal)
                thirty = scaled_window_change(hist, nid, now, 30.0, op_bal)

                row = {
                    "node_id": nid,
                    "hostname": hostname,
                    "identity_key": identity_key,
                    "wallet": wallet_addr,
                    "uptime": uptime,
                    "version": extract_version_from_desc(d),
                    "operator_balance": op_bal,
                    "profit_difference": diff,
                    "epochs": hours,
                    "average_hour": (diff / hours) if hours > 0 else 0.0,
                    "7_days": window_cell(seven, 0, 7),
                    "7_days_average": window_cell(seven, 2, 7),
                    "30_days": window_cell(thirty, 0, 30),
                    "30_days_average": window_cell(thirty, 2, 30),
                    "tag": tag,
                    "members": members,
                    "your_share": op_bal / members,
                    "_prev_balance": prev_bal,
                    "_prev_ts": prev_ts,
                    "_wallet_order": wallet_order.get(wallet_addr, 10**9),
                }

                # append current snapshot & prune >30d
                add_history_point(hist, nid, now, uptime, op_bal)

                out_rows.append(row)
                rows_by_tag[tag].append(row)
            continue

        # path from /nodes
        for n in wallet_nodes:
            nid = to_int(n.get("node_id"), 0)
            if nid <= 0:
                continue
            identity_key = n.get("identity_key") or ""
            uptime = to_float(n.get("uptime"), 0.0)
            desc = n.get("description") or {}
            hostinfo = desc.get("host_information") or {}
            hostname = hostinfo.get("hostname") or ""
            op_unym = to_float(n.get("rewarding_details", {}).get("operator"), 0.0)
            op_bal = op_unym / 1_000_000.0
            if op_bal <= 0 and args.fetch_wallet_balances:
                op_bal = fetch_balance_total_nym(wallet_addr)

            prev = last_snapshot(hist, nid)
            prev_bal = to_float(prev.get("operator_balance"), 0.0) if prev else None
            prev_ts = to_float(prev.get("ts"), 0.0) if prev else None

            diff = 0.0
            hours = 0.0
            if prev is not None:
                diff = op_bal - prev_bal
                hours = max(0.0, (now - prev_ts) / 3600.0)

            seven = scaled_window_change(hist, nid, now, 7.0, op_bal)
            thirty = scaled_window_change(hist, nid, now, 30.0, op_bal)

            row = {
                "node_id": nid,
                "hostname": hostname,
                "identity_key": identity_key,
                "wallet": wallet_addr,
                "uptime": uptime,
                "version": extract_version_from_node(n),
                "operator_balance": op_bal,
                "profit_difference": diff,
                "epochs": hours,
                "average_hour": (diff / hours) if hours > 0 else 0.0,
                "7_days": window_cell(seven, 0, 7),
                "7_days_average": window_cell(seven, 2, 7),
                "30_days": window_cell(thirty, 0, 30),
                "30_days_average": window_cell(thirty, 2, 30),
                "tag": tag,
                "members": members,
                "your_share": op_bal / members,
                "_prev_balance": prev_bal,
                "_prev_ts": prev_ts,
                "_wallet_order": wallet_order.get(wallet_addr, 10**9),
            }

            add_history_point(hist, nid, now, uptime, op_bal)

            out_rows.append(row)
            rows_by_tag[tag].append(row)

    # final prune & save — prune to the retention buffer (not the 30d window)
    # so the comparison point just outside 30 days survives into the next run.
    # Skipped entirely when writes are disabled: a read-only query must not
    # record a snapshot, and must never risk clobbering data.yaml if it runs
    # concurrently with a scheduled full run.
    if writes_enabled:
        cleanup_history_older_than(hist, now - RETENTION_SEC)
        save_history(HIST_FILE, hist)

    # write CSV (skipped in read-only mode)
    if writes_enabled:
        headers_csv = [
            "node_id","hostname","identity_key","wallet","uptime","version","operator_balance",
            "profit_difference","epochs","average_hour","7_days","7_days_average",
            "30_days","30_days_average","tag","members","your_share",
        ]
        with open(OUT_CSV, "w", newline="", encoding="utf-8") as f:
            writer = csv.DictWriter(f, fieldnames=headers_csv)
            writer.writeheader()
            for r in out_rows:
                writer.writerow({
                    "node_id": r.get("node_id",""),
                    "hostname": r.get("hostname",""),
                    "identity_key": r.get("identity_key",""),
                    "wallet": r.get("wallet",""),
                    "uptime": f"{to_float(r.get('uptime'),0.0):.6f}",
                    "version": r.get("version",""),
                    "operator_balance": f"{to_float(r.get('operator_balance'),0.0):.6f}",
                    "profit_difference": f"{to_float(r.get('profit_difference'),0.0):.6f}",
                    "epochs": f"{to_float(r.get('epochs'),0.0):.6f}",
                    "average_hour": f"{to_float(r.get('average_hour'),0.0):.6f}",
                    "7_days": r.get("7_days",""),
                    "7_days_average": r.get("7_days_average",""),
                    "30_days": r.get("30_days",""),
                    "30_days_average": r.get("30_days_average",""),
                    "tag": r.get("tag",""),
                    "members": r.get("members", 1),
                    "your_share": f"{to_float(r.get('your_share'),0.0):.6f}",
                })

    if not out_rows:
        log(f"{Fore.YELLOW}No rows produced — check inputs and endpoints.{Style.RESET_ALL}")
        return

    # accumulate across entities for the final summary
    grand_total = 0.0
    grand_share = 0.0
    entity_lines: List[Tuple[str, int, int, float, float]] = []

    # per-tag output (preserve input order)
    for tag, rows in sorted(rows_by_tag.items(), key=lambda kv: kv[0] or ""):
        headers_print = [
            "node_id","hostname","wallet","uptime","version",
            "operator_balance","profit_difference","epochs","average_hour",
            "7_days","7_days_average","30_days","30_days_average","your_share",
        ]
        view: List[List[str]] = []
        for r in rows:
            u = to_float(r.get("uptime"), 0.0)
            u_col = uptime_color_name(u)
            view.append([
                r.get("node_id") or "",
                (r.get("hostname") or "")[:40],
                r.get("wallet") or "",
                colorize(f"{u:.2f}", u_col) if u_col else f"{u:.2f}",
                r.get("version") or "",
                f"{to_float(r.get('operator_balance'), 0.0):.2f}",
                f"{to_float(r.get('profit_difference'), 0.0):.2f}",
                f"{to_float(r.get('epochs'), 0.0):.2f}",
                f"{to_float(r.get('average_hour'), 0.0):.2f}",
                r.get("7_days"),
                r.get("7_days_average"),
                r.get("30_days"),
                r.get("30_days_average"),
                f"{to_float(r.get('your_share'), 0.0):.2f}",
            ])

        title = f"Tag: {tag or '(untagged)'}   —   {len(rows)} node(s)"
        if not RAW_MODE:
            print("\n" + title)
            print(tabulate(view, headers=headers_print, tablefmt="github",
                           stralign="right", disable_numparse=True))

        # per-tag summary
        tag_total_now = sum(to_float(r.get("operator_balance"), 0.0) for r in rows)
        tag_members = members_by_tag.get(tag, 1)
        tag_share_now = tag_total_now / tag_members
        grand_total += tag_total_now
        grand_share += tag_share_now
        entity_lines.append((tag or "(untagged)", len(rows), tag_members,
                             tag_total_now, tag_share_now))

        # since last time: sum diffs, average hours across nodes that existed
        prev_sum = 0.0
        prev_hours: List[float] = []
        for r in rows:
            prev_bal = r.get("_prev_balance")
            prev_ts = r.get("_prev_ts")
            if prev_bal is not None and prev_ts is not None:
                prev_sum += to_float(prev_bal, 0.0)
                prev_hours.append(max(0.0, (now - to_float(prev_ts, now)) / 3600.0))
        diff_total = tag_total_now - prev_sum if prev_hours else 0.0
        hours_since = (sum(prev_hours) / len(prev_hours)) if prev_hours else 0.0
        hourly = (diff_total / hours_since) if hours_since > 0 else 0.0

        # 7-day / 30-day per-tag totals: sum nodes with data; hours fixed windows if any
        def _num_or_none(v):
            try:
                return float(v)
            except Exception:
                return None
        seven_vals = [_num_or_none(r.get("7_days")) for r in rows]
        seven_vals = [v for v in seven_vals if v is not None]
        total7 = sum(seven_vals) if seven_vals else 0.0
        hours7 = 7.0 * 24.0 if seven_vals else 0.0
        hourly7 = (total7 / hours7) if hours7 > 0 else 0.0

        thirty_vals = [_num_or_none(r.get("30_days")) for r in rows]
        thirty_vals = [v for v in thirty_vals if v is not None]
        total30 = sum(thirty_vals) if thirty_vals else 0.0
        hours30 = 30.0 * 24.0 if thirty_vals else 0.0
        hourly30 = (total30 / hours30) if hours30 > 0 else 0.0

        # print output with colored numbers
        if not RAW_MODE:
            print(
            "\n"
            f"Total balance across all wallets: {Style.BRIGHT}{Fore.GREEN}{tag_total_now:.2f}{Style.RESET_ALL} NYM\n"
            f"Your share (1/{tag_members}): {Style.BRIGHT}{Fore.YELLOW}{tag_share_now:.2f}{Style.RESET_ALL} NYM\n"
            f"Difference of total balance from last time: {Fore.CYAN}{diff_total:.2f}{Style.RESET_ALL} NYM\n"
            f"Time since last time: {Fore.BLUE}{hours_since:.2f}{Style.RESET_ALL} hours\n"
            f"Approx hourly difference: {Style.BRIGHT}{Fore.GREEN}{hourly:.2f}{Style.RESET_ALL} NYM/h\n"
            f"7-day change: "
            f"{('no 7 days data stored' if not seven_vals else f'{Fore.CYAN}{total7:.2f}{Style.RESET_ALL} NYM, hours: {Fore.BLUE}{hours7:.2f}{Style.RESET_ALL}, hourly: {Style.BRIGHT}{Fore.GREEN}{hourly7:.2f}{Style.RESET_ALL} NYM/h')}\n"
            f"30-day change: "
            f"{('no 30 days data stored' if not thirty_vals else f'{Fore.CYAN}{total30:.2f}{Style.RESET_ALL} NYM, hours: {Fore.BLUE}{hours30:.2f}{Style.RESET_ALL}, hourly: {Style.BRIGHT}{Fore.GREEN}{hourly30:.2f}{Style.RESET_ALL} NYM/h')}\n"
        )

    # grand summary across every entity
    if entity_lines and not RAW_MODE:
        print("\n" + "=" * 72)
        print(f"{Style.BRIGHT}SUMMARY - ALL ENTITIES{Style.RESET_ALL}")
        print("=" * 72)
        summary_rows = [
            [name, nodes_n, mem, f"{tot:.2f}", f"{shr:.2f}"]
            for name, nodes_n, mem, tot, shr in entity_lines
        ]
        print(tabulate(
            summary_rows,
            headers=["entity", "nodes", "members", "total NYM", "your share NYM"],
            tablefmt="github",
            stralign="right",
            disable_numparse=True,
        ))
        print(
            "\n"
            f"Total rewards on all nodes:      "
            f"{Style.BRIGHT}{Fore.GREEN}{grand_total:.2f}{Style.RESET_ALL} NYM\n"
            f"Total your share (after 1/N):    "
            f"{Style.BRIGHT}{Fore.YELLOW}{grand_share:.2f}{Style.RESET_ALL} NYM"
        )
        if not args.fetch_wallet_balances:
            print(
                f"\n{Fore.BLUE}Bulk API mode: no per-wallet requests were made, "
                f"so your wallets cannot be correlated by the API operator.\n"
                f"Use --fetch-wallet-balances only if you accept that trade-off."
                f"{Style.RESET_ALL}"
            )

    # --raw: exactly one number on stdout, nothing else, for piping
    if args.raw:
        value = grand_total if args.raw == "total" else grand_share
        print(f"{value:.6f}")
        return

    if writes_enabled:
        log(f"\nCSV written to: {OUT_CSV}")
        log(f"History saved to: {HIST_FILE}")
    else:
        log(f"\n{Fore.BLUE}Read-only run: no files in {DATA_DIR} were "
            f"changed.{Style.RESET_ALL}")


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        sys.exit(130)
        