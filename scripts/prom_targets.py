import argparse
import requests
import json


def make_prom_target(mixnode):
    bond_info = mixnode.get("bond_information", {})
    mix_node = bond_info.get("mix_node", {})
    host = mix_node.get("host", None)
    port = mix_node.get("http_api_port", None)
    if host is None or port is None:
        return None

    return {
        "targets": [f"{host}:{port}"],
        "labels": {
            "host": host,
            "identity_key": mix_node.get("identity_key", None),
            "sphinx_key": mix_node.get("sphinx_key", None),
            "version": mix_node.get("version", None),
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
    outfile = "/tmp/prom_targets.json"

    mixnodes = requests.get(f"{nym_api}/api/v1/mixnodes").json()
    prom_targets = [make_prom_target(mixnode) for mixnode in mixnodes]
    j = json.dumps(prom_targets)
    with open(outfile, "w") as fp:
        fp.write(j)
    print(f"Prometheus -> {len(prom_targets)} targets written to {outfile}")
