import argparse
import os
import requests
import json
from collections import namedtuple

Config = namedtuple("Config", ["port", "outfile", "outlink", "env"])


def validate_config_entry(entry):
    return entry.get("nym_api") and entry.get("env")


def config_to_targets(config, mixnodes):
    prom_targets = [
        make_prom_target(config.env, mixnode, config.port) for mixnode in mixnodes
    ]
    with open(config.outfile, "w") as f:
        json.dump(prom_targets, f)

    os.chmod(config.outfile, 0o777)
    os.rename(config.outfile, config.outlink)
    os.chmod(config.outlink, 0o777)

    print(f"Prometheus -> {len(prom_targets)} targets written to {config.outlink}")


def make_prom_target(env, mixnode, port=None):
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
            "mixnet_env": env,
        },
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="Create prometheus targets for rewarded set mixnodes."
    )
    parser.add_argument(
        "config",
        type=str,
        help="Config file, see scripts/prom_targets_config.json",
    )

    args = parser.parse_args()
    config_file = args.config

    with open(config_file, "r") as f:
        config = json.load(f)

    for entry in config:

        if not validate_config_entry(entry):
            print(f"Invalid config entry: {entry}")
            continue

        targets = [
            Config(
                None,
                "/tmp/temp_targets_mix.json",
                f"/tmp/prom_targets_mix_{entry['env']}.json",
                entry["env"],
            ),
            Config(
                9100,
                "/tmp/temp_targets_node.json",
                f"/tmp/prom_targets_node_{entry['env']}.json",
                entry["env"],
            ),
        ]

        mixnodes = requests.get(f"{entry['nym_api']}/api/v1/mixnodes").json()

        for config in targets:
            config_to_targets(config, mixnodes)
