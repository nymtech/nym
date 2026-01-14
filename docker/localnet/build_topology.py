import json
import os
import subprocess
import sys
from datetime import datetime
from functools import lru_cache
from pathlib import Path

import base58

DEFAULT_OWNER = "n1jw6mp7d5xqc7w6xm79lha27glmd0vdt3l9artf"
DEFAULT_SUFFIX = os.environ.get("NYM_NODE_SUFFIX", "localnet")
NYM_NODES_ROOT = Path.home() / ".nym" / "nym-nodes"


def debug(msg):
    """Print debug message to stderr"""
    print(f"[DEBUG] {msg}", file=sys.stderr, flush=True)


def error(msg):
    """Print error message to stderr"""
    print(f"[ERROR] {msg}", file=sys.stderr, flush=True)


def maybe_assign(target, key, value):
    if value is not None:
        target[key] = value


@lru_cache(maxsize=None)
def get_nym_node_version():
    try:
        result = subprocess.run(
            ["nym-node", "--version"],
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
    except (subprocess.CalledProcessError, FileNotFoundError):
        return None

    version_line = result.stdout.strip()
    if not version_line:
        return None

    parts = version_line.split()
    for token in reversed(parts):
        if token and token[0].isdigit():
            return token
    return version_line


def node_config_path(prefix, suffix):
    path = NYM_NODES_ROOT / f"{prefix}-{suffix}" / "config" / "config.toml"
    debug(f"Looking for config at: {path}")
    if path.exists():
        debug(f"  ✓ Config found")
        return path
    else:
        error(f"  ✗ Config NOT found at {path}")
        return None


def read_node_details(prefix, suffix):
    config_path = node_config_path(prefix, suffix)
    if config_path is None:
        error(f"Cannot read node details for {prefix}-{suffix}: config not found")
        return {}

    debug(f"Running: nym-node node-details --config-file {config_path}")
    try:
        result = subprocess.run(
            [
                "nym-node",
                "node-details",
                "--config-file",
                str(config_path),
                "--output=json",
            ],
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        debug(f"  ✓ node-details command succeeded")
    except subprocess.CalledProcessError as e:
        error(f"node-details command failed for {prefix}-{suffix}: {e}")
        error(f"  stdout: {e.stdout}")
        error(f"  stderr: {e.stderr}")
        return {}
    except FileNotFoundError:
        error("nym-node command not found in PATH")
        return {}

    try:
        details = json.loads(result.stdout)
        debug(f"  ✓ Parsed node-details JSON")
    except json.JSONDecodeError as e:
        error(f"Failed to parse node-details JSON: {e}")
        error(f"  Output was: {result.stdout[:200]}")
        return {}

    info = {}

    # Get sphinx key and decode from Base58 to byte array
    sphinx_data = details.get("x25519_primary_sphinx_key")
    if isinstance(sphinx_data, dict):
        sphinx_key_b58 = sphinx_data.get("public_key")
        if sphinx_key_b58:
            debug(f"  Got sphinx_key (Base58): {sphinx_key_b58[:20]}...")
            try:
                # Decode Base58 to byte array
                sphinx_bytes = base58.b58decode(sphinx_key_b58)
                info["sphinx_key"] = list(sphinx_bytes)
                debug(f"  ✓ Decoded to {len(sphinx_bytes)} bytes")
            except Exception as e:
                error(f"  Failed to decode sphinx_key: {e}")

    version = get_nym_node_version()
    if version:
        info["version"] = version

    return info


def resolve_host(data):
    # For localnet, always use 127.0.0.1 unless explicitly overridden
    env_host = os.environ.get("LOCALNET_PUBLIC_IP") or os.environ.get("NYMNODE_PUBLIC_IP")
    if env_host:
        return env_host.split(",")[0].strip()

    # Default to localhost for localnet (containers can reach each other via published ports)
    return "127.0.0.1"


def create_mixnode_entry(base_dir, mix_id, port_delta, suffix, host_ip):
    """Create a node_details entry for a mixnode"""
    debug(f"\n=== Creating mixnode{mix_id} entry ===")
    mix_file = Path(base_dir) / f"mix{mix_id}.json"
    debug(f"Reading bonding JSON from: {mix_file}")
    with mix_file.open("r") as json_blob:
        mix_data = json.load(json_blob)

    node_details = read_node_details(f"mix{mix_id}", suffix)

    # Get identity key from bonding JSON (already byte array)
    identity = mix_data.get("identity_key")
    if not identity:
        raise RuntimeError(f"Missing identity_key in {mix_file}")
    debug(f"  ✓ Got identity_key from bonding JSON: {len(identity)} bytes")

    # Get sphinx key from node-details (decoded from Base58)
    sphinx_key = node_details.get("sphinx_key")
    if not sphinx_key:
        raise RuntimeError(f"Missing sphinx_key from node-details for mix{mix_id}")

    host = host_ip
    port = 10000 + port_delta
    debug(f"  Using host: {host}:{port}")

    entry = {
        "node_id": mix_id,
        "mix_host": f"{host}:{port}",
        "entry": None,
        "identity_key": identity,
        "sphinx_key": sphinx_key,
        "supported_roles": {
            "mixnode": True,
            "mixnet_entry": False,
            "mixnet_exit": False
        }
    }

    maybe_assign(entry, "version", node_details.get("version") or mix_data.get("version"))

    return entry


def create_gateway_entry(base_dir, node_id, port_delta, suffix, host_ip, gateway_name="gateway"):
    """Create a node_details entry for a gateway"""
    debug(f"\n=== Creating {gateway_name} entry ===")
    gateway_file = Path(base_dir) / f"{gateway_name}.json"
    debug(f"Reading bonding JSON from: {gateway_file}")
    with gateway_file.open("r") as json_blob:
        gateway_data = json.load(json_blob)

    node_details = read_node_details(gateway_name, suffix)

    # Get identity key from bonding JSON (already byte array)
    identity = gateway_data.get("identity_key")
    if not identity:
        raise RuntimeError(f"Missing identity_key in {gateway_name}.json")
    debug(f"  ✓ Got identity_key from bonding JSON: {len(identity)} bytes")

    # Get sphinx key from node-details (decoded from Base58)
    sphinx_key = node_details.get("sphinx_key")
    if not sphinx_key:
        raise RuntimeError(f"Missing sphinx_key from node-details for {gateway_name}")

    host = host_ip
    mix_port = 10000 + port_delta
    # Calculate clients_port: gateway uses 9000, gateway2 uses 9001, etc.
    clients_port = 9000 + (port_delta - 4)
    debug(f"  Using host: {host} (mix:{mix_port}, clients:{clients_port})")

    entry = {
        "node_id": node_id,
        "mix_host": f"{host}:{mix_port}",
        "entry": {
            "ip_addresses": [host],
            "clients_ws_port": clients_port,
            "hostname": None,
            "clients_wss_port": None
        },
        "identity_key": identity,
        "sphinx_key": sphinx_key,
        "supported_roles": {
            "mixnode": False,
            "mixnet_entry": True,
            "mixnet_exit": True
        }
    }

    maybe_assign(entry, "version", node_details.get("version") or gateway_data.get("version"))

    return entry


def main(args):
    if not args:
        raise SystemExit("Usage: build_topology.py <output_dir> [node_suffix] [mix1_ip] [mix2_ip] [mix3_ip] [gateway_ip] [gateway2_ip]")

    base_dir = args[0]
    suffix = args[1] if len(args) > 1 and args[1] else DEFAULT_SUFFIX

    # Get container IPs from arguments (or use 127.0.0.1 as fallback)
    mix1_ip = args[2] if len(args) > 2 else "127.0.0.1"
    mix2_ip = args[3] if len(args) > 3 else "127.0.0.1"
    mix3_ip = args[4] if len(args) > 4 else "127.0.0.1"
    gateway_ip = args[5] if len(args) > 5 else "127.0.0.1"
    gateway2_ip = args[6] if len(args) > 6 else "127.0.0.1"

    debug(f"\n=== Starting topology generation ===")
    debug(f"Output directory: {base_dir}")
    debug(f"Node suffix: {suffix}")
    debug(f"Container IPs: mix1={mix1_ip}, mix2={mix2_ip}, mix3={mix3_ip}, gateway={gateway_ip}, gateway2={gateway2_ip}")

    # Create node_details entries with integer keys
    node_details = {
        1: create_mixnode_entry(base_dir, 1, 1, suffix, mix1_ip),
        2: create_mixnode_entry(base_dir, 2, 2, suffix, mix2_ip),
        3: create_mixnode_entry(base_dir, 3, 3, suffix, mix3_ip),
        4: create_gateway_entry(base_dir, 4, 4, suffix, gateway_ip, "gateway"),
        5: create_gateway_entry(base_dir, 5, 5, suffix, gateway2_ip, "gateway2")
    }

    # Create the NymTopology structure
    topology = {
        "metadata": {
            "key_rotation_id": 0,
            "absolute_epoch_id": 0,
            "refreshed_at": datetime.utcnow().isoformat() + "Z"
        },
        "rewarded_set": {
            "epoch_id": 0,
            "entry_gateways": [4, 5],
            "exit_gateways": [4, 5],
            "layer1": [1],
            "layer2": [2],
            "layer3": [3],
            "standby": []
        },
        "node_details": node_details
    }

    output_path = Path(base_dir) / "network.json"
    debug(f"\nWriting topology to: {output_path}")
    with output_path.open("w") as out:
        json.dump(topology, out, indent=2)

    print(f"✓ Generated topology with {len(node_details)} nodes")
    print(f"  - 3 mixnodes (layers 1, 2, 3)")
    print(f"  - 2 gateways (entry + exit)")
    debug(f"\n=== Topology generation complete ===\n")


if __name__ == "__main__":
    main(sys.argv[1:])
