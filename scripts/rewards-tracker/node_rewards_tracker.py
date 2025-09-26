#!/usr/bin/env python3
# -*- coding: utf-8 -*-

"""
This script fetches operators rewards based on provided Nyx account addresses provided in data/wallet-addresses.csv.
Output is:
    1. Printet table in terminal
    2. Sheet with complete info stored in data/node-balances.csv
    3. Hiostorical data yaml file stored in data/data.yaml - this file should not be changed by hand, as
    all values older than 30 days get auto-removed
Before you start fill first column of data/wallet-addresses with your Nyx account addresses and (optionally) second column
with a tag, for example "mysquad" and "personal" to get sorted output per entity.
"""

import csv
import os
import sys
import time
import yaml
import requests
from collections import defaultdict
from typing import Any, Dict, List, Tuple, Optional
from tabulate import tabulate
from colorama import init as colorama_init, Fore, Style

colorama_init()

DATA_DIR = os.path.join(os.getcwd(), "data")
ADDR_CSV = os.path.join(DATA_DIR, "wallet-addresses.csv")
OUT_CSV = os.path.join(DATA_DIR, "node-balances.csv")
HIST_FILE = os.path.join(DATA_DIR, "data.yaml")

SPECTRE_NODES_URL = "https://api.nym.spectredao.net/api/v1/nodes"
VALIDATOR_BONDED_URL = "https://validator.nymtech.net/api/v1/nym-nodes/bonded"
VALIDATOR_DESC_URL = "https://validator.nymtech.net/api/v1/nym-nodes/described"
SPECTRE_BAL_URL = "https://api.nym.spectredao.net/api/v1/balances/{address}"

SESSION = requests.Session()
SESSION.headers.update({"User-Agent": "nym-tools/1.0"})


def log(msg: str) -> None:
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

def _fetch_all_limit_offset(url: str, limit: int = 1000, timeout: int = 60) -> List[Any]:
    out: List[Any] = []
    offset = 0
    tries = 0
    while True:
        tries += 1
        data = _get_json(url, {"limit": limit, "offset": offset}, timeout=timeout)
        if isinstance(data, dict) and "data" in data and isinstance(data["data"], list):
            items = data["data"]
        elif isinstance(data, list):
            items = data
        else:
            break
        if not items:
            break
        out.extend(items)
        if len(items) < limit:
            break
        offset += limit
        if tries > 500:
            break
    return out

def _fetch_all_page_pagesize(url: str, page_size: int = 1000, timeout: int = 60) -> List[Any]:
    out: List[Any] = []
    page = 0
    tries = 0
    while True:
        tries += 1
        data = _get_json(url, {"page": page, "size": page_size}, timeout=timeout)
        if isinstance(data, dict) and "data" in data and isinstance(data["data"], list):
            items = data["data"]
        elif isinstance(data, list):
            items = data
        else:
            break
        out.extend(items)
        total = to_int(data.get("pagination", {}).get("total"), -1) if isinstance(data, dict) else -1
        if total >= 0 and len(out) >= total:
            break
        if not items:
            break
        page += 1
        if tries > 500:
            break
    return out

def _fetch_all_single(url: str, timeout: int = 60) -> List[Any]:
    data = _get_json(url, {}, timeout=timeout)
    if isinstance(data, dict) and "data" in data and isinstance(data["data"], list):
        return data["data"]
    if isinstance(data, list):
        return data
    return []

def _fetch_all_any(url: str, timeout: int = 60) -> list:
    got = _fetch_all_limit_offset(url, limit=1000, timeout=timeout)
    if isinstance(got, list) and got:
        return got
    got = _fetch_all_page_pagesize(url, page_size=1000, timeout=timeout)
    if isinstance(got, list) and got:
        return got
    return _fetch_all_single(url, timeout=timeout)


# load data

def read_wallets_csv(path: str) -> List[Tuple[str, str]]:
    if not os.path.exists(path):
        raise FileNotFoundError(f"Input CSV not found: {path}")
    rows: List[Tuple[str, str]] = []
    with open(path, newline="", encoding="utf-8") as f:
        reader = csv.DictReader(f)
        for row in reader:
            addr = (row.get("address") or "").strip()
            tag = (row.get("tag") or "").strip()
            if addr:
                rows.append((addr, tag))
    return rows

def fetch_nodes_all() -> List[Dict[str, Any]]:
    log(f"* * * Fetching nodes from {SPECTRE_NODES_URL} * * *")
    nodes = _fetch_all_any(SPECTRE_NODES_URL, timeout=90)
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
    if not os.path.exists(path):
        return {}
    with open(path, "r", encoding="utf-8") as f:
        try:
            return yaml.safe_load(f) or {}
        except Exception:
            return {}

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
) -> Optional[Tuple[float, float, float]]:
    key = str(node_id)
    if key not in hist or not hist[key]:
        return None
    cutoff_ts = now - window_days * 24 * 3600
    candidates = [e for e in hist[key] if to_float(e.get("ts"), 0.0) <= cutoff_ts]
    if not candidates:
        return None
    snap = sorted(candidates, key=lambda x: x.get("ts", 0.0))[-1]
    span_hours = max(0.0, (now - to_float(snap.get("ts"), now)) / 3600.0)
    if span_hours <= 0:
        return None
    profit_raw = current_balance - to_float(snap.get("operator_balance"), 0.0)
    target_hours = window_days * 24.0
    profit_scaled = profit_raw * (target_hours / span_hours)
    hourly_scaled = profit_scaled / target_hours
    return profit_scaled, span_hours, hourly_scaled


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

def main() -> None:
    os.makedirs(DATA_DIR, exist_ok=True)

    wallets = read_wallets_csv(ADDR_CSV)
    if not wallets:
        log(f"{Fore.RED}No wallets found in {ADDR_CSV}{Style.RESET_ALL}")
        sys.exit(1)
    log(f"Found {len(wallets)} wallet(s) in {ADDR_CSV}")

    # preserve input order per wallet for per-tag tables
    wallet_order = {addr: idx for idx, (addr, _tag) in enumerate(wallets)}

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
    ]

    hist = load_history(HIST_FILE)
    now = now_ts()

    out_rows: List[Dict[str, Any]] = []
    rows_by_tag: Dict[str, List[Dict[str, Any]]] = defaultdict(list)

    THIRTY_DAYS_SEC = 30 * 24 * 3600
    cleanup_history_older_than(hist, now - THIRTY_DAYS_SEC)  # prune before use

    for wallet_addr, tag in wallets:
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

                op_bal = fetch_balance_total_nym(wallet_addr)
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
                    "7_days": f"{seven[0]:.6f}" if seven else "no 7 days data stored",
                    "7_days_average": f"{seven[2]:.6f}" if seven else "no 7 days data stored",
                    "30_days": f"{thirty[0]:.6f}" if thirty else "no 30 days data stored",
                    "30_days_average": f"{thirty[2]:.6f}" if thirty else "no 30 days data stored",
                    "tag": tag,
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
            if op_bal <= 0:
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
                "7_days": f"{seven[0]:.6f}" if seven else "no 7 days data stored",
                "7_days_average": f"{seven[2]:.6f}" if seven else "no 7 days data stored",
                "30_days": f"{thirty[0]:.6f}" if thirty else "no 30 days data stored",
                "30_days_average": f"{thirty[2]:.6f}" if thirty else "no 30 days data stored",
                "tag": tag,
                "_prev_balance": prev_bal,
                "_prev_ts": prev_ts,
                "_wallet_order": wallet_order.get(wallet_addr, 10**9),
            }

            add_history_point(hist, nid, now, uptime, op_bal)

            out_rows.append(row)
            rows_by_tag[tag].append(row)

    # final prune & save
    cleanup_history_older_than(hist, now - THIRTY_DAYS_SEC)
    save_history(HIST_FILE, hist)

    # write CSV
    headers_csv = [
        "node_id","hostname","identity_key","wallet","uptime","version","operator_balance",
        "profit_difference","epochs","average_hour","7_days","7_days_average",
        "30_days","30_days_average","tag",
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
            })

    if not out_rows:
        log(f"{Fore.YELLOW}No rows produced — check inputs and endpoints.{Style.RESET_ALL}")
        return

    # per-tag output (preserve input order)
    for tag, rows in sorted(rows_by_tag.items(), key=lambda kv: kv[0] or ""):
        headers_print = [
            "node_id","hostname","wallet","uptime","version",
            "operator_balance","profit_difference","epochs","average_hour",
            "7_days","7_days_average","30_days","30_days_average",
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
            ])

        title = f"Tag: {tag or '(untagged)'}   —   {len(rows)} node(s)"
        print("\n" + title)
        print(tabulate(view, headers=headers_print, tablefmt="github", stralign="right", disable_numparse=True))

        # per-tag summary
        tag_total_now = sum(to_float(r.get("operator_balance"), 0.0) for r in rows)

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
        print(
            "\n"
            f"Total balance across all wallets: {Style.BRIGHT}{Fore.GREEN}{tag_total_now:.2f}{Style.RESET_ALL} NYM\n"
            f"Difference of total balance from last time: {Fore.CYAN}{diff_total:.2f}{Style.RESET_ALL} NYM\n"
            f"Time since last time: {Fore.BLUE}{hours_since:.2f}{Style.RESET_ALL} hours\n"
            f"Approx hourly difference: {Style.BRIGHT}{Fore.GREEN}{hourly:.2f}{Style.RESET_ALL} NYM/h\n"
            f"7-day change: "
            f"{('no 7 days data stored' if not seven_vals else f'{Fore.CYAN}{total7:.2f}{Style.RESET_ALL} NYM, hours: {Fore.BLUE}{hours7:.2f}{Style.RESET_ALL}, hourly: {Style.BRIGHT}{Fore.GREEN}{hourly7:.2f}{Style.RESET_ALL} NYM/h')}\n"
            f"30-day change: "
            f"{('no 30 days data stored' if not thirty_vals else f'{Fore.CYAN}{total30:.2f}{Style.RESET_ALL} NYM, hours: {Fore.BLUE}{hours30:.2f}{Style.RESET_ALL}, hourly: {Style.BRIGHT}{Fore.GREEN}{hourly30:.2f}{Style.RESET_ALL} NYM/h')}\n"
        )

    log(f"\nCSV written to: {OUT_CSV}")
    log(f"History saved to: {HIST_FILE}")


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print("\nInterrupted.", file=sys.stderr)
        sys.exit(130)
