import argparse
import os
import requests
import json
from datetime import datetime
from collections import namedtuple

Config = namedtuple("Config", ["port", "outfile", "outlink"])


def config_to_targets(config, mixnodes):
    prom_targets = [make_prom_target(mixnode, config.port) for mixnode in mixnodes]
    with open(config.outfile, "w") as f:
        json.dump(prom_targets, f)

    os.chmod(config.outfile, 0o777)
    os.rename(config.outfile, config.outlink)
    os.chmod(config.outlink, 0o777)

    print(f"Prometheus -> {len(prom_targets)} targets written to {config.outlink}")


def make_prom_target(mixnode, port=None):
    bond_info = mixnode.get("bond_information", {})
    mix_node = bond_info.get("mix_node", {})
    host = mix_node.get("host", None)
    port = port if port else mix_node.get("http_api_port", None)
    if host is None or port is None:
        return None

    return {
        "targets": [f"{host}:{port}"],
        "labels": {
            "mix_node_host": host,
            "identity_key": mix_node.get("identity_key", None),
            "sphinx_key": mix_node.get("sphinx_key", None),
            "mix_node_version": mix_node.get("version", None),
        },
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="Create prometheus targets for rewarded set mixnodes."
    )
    parser.add_argument(
        "apiurl",
        type=str,
        help="Nym Api url",
    )

    args = parser.parse_args()
    nym_api = args.apiurl

    targets = [
        Config(None, "/tmp/temp_targets_mix.json", "/tmp/prom_targets_mix.json"),
        Config(9100, "/tmp/temp_targets_node.json", "/tmp/prom_targets_node.json"),
    ]

    mixnodes = requests.get(f"{nym_api}/api/v1/mixnodes").json()

    for config in targets:
        config_to_targets(config, mixnodes)
