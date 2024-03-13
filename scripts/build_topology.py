import json
import os.path
import sys


def add_mixnode(base_network, base_dir, mix_id):
    with open(os.path.join(base_dir, "mix" + str(mix_id) + ".json"), "r") as json_blob:
        mix_data = json.load(json_blob)
        base_network["mixnodes"][str(mix_id)][0]["identity_key"] = mix_data["identity_key"]
        base_network["mixnodes"][str(mix_id)][0]["sphinx_key"] = mix_data["sphinx_key"]
        base_network["mixnodes"][str(mix_id)][0]["mix_port"] = mix_data["mix_port"]
        base_network["mixnodes"][str(mix_id)][0]["version"] = mix_data["version"]
        base_network["mixnodes"][str(mix_id)][0]["host"] = mix_data["bind_address"]
        base_network["mixnodes"][str(mix_id)][0]["layer"] = mix_id
        base_network["mixnodes"][str(mix_id)][0]["mix_id"] = mix_id
        base_network["mixnodes"][str(mix_id)][0]["owner"] = "whatever"

        #described_node
        template = mixnode_template()
        template["Mixnode"]["bond"]["mix_node"]["identity_key"] = mix_data["identity_key"]
        template["Mixnode"]["bond"]["mix_node"]["sphinx_key"] = mix_data["sphinx_key"]
        template["Mixnode"]["bond"]["mix_node"]["mix_port"] = mix_data["mix_port"]
        template["Mixnode"]["bond"]["mix_node"]["host"] = mix_data["bind_address"]
        template["Mixnode"]["bond"]["layer"] = mix_id
        template["Mixnode"]["bond"]["mix_id"] = mix_id
        template["Mixnode"]["self_described"]["host_information"]["keys"]["ed25519"] = mix_data["identity_key"]
        template["Mixnode"]["self_described"]["host_information"]["keys"]["x25519"] = mix_data["sphinx_key"]
        base_network["describedNodes"][mix_id] = template
        return base_network


def add_gateway(base_network, base_dir):
    with open(os.path.join(base_dir, "gateway.json"), "r") as json_blob:
        gateway_data = json.load(json_blob)
        base_network["gateways"][0]["identity_key"] = gateway_data["identity_key"]
        base_network["gateways"][0]["sphinx_key"] = gateway_data["sphinx_key"]
        base_network["gateways"][0]["mix_port"] = gateway_data["mix_port"]
        base_network["gateways"][0]["clients_port"] = gateway_data["clients_port"]
        # base_network["gateways"][0]["version"] = gateway_data["version"]
        base_network["gateways"][0]["host"] = gateway_data["bind_address"]
        base_network["gateways"][0]["owner"] = "whatever"

        #described_node
        template = gateway_template()
        template["Gateway"]["bond"]["gateway"]["identity_key"] = gateway_data["identity_key"]
        template["Gateway"]["bond"]["gateway"]["sphinx_key"] = gateway_data["sphinx_key"]
        template["Gateway"]["bond"]["gateway"]["mix_port"] = gateway_data["mix_port"]
        template["Gateway"]["bond"]["gateway"]["clients_port"] = gateway_data["clients_port"]
        template["Gateway"]["bond"]["gateway"]["host"] = gateway_data["bind_address"]
        template["Gateway"]["self_described"]["host_information"]["keys"]["ed25519"] = gateway_data["identity_key"]
        template["Gateway"]["self_described"]["host_information"]["keys"]["x25519"] = gateway_data["sphinx_key"]
        base_network["describedNodes"][0] = template
        return base_network


def main(args):
    base_network = {
        "mixnodes": {
            "1": [{}],
            "2": [{}],
            "3": [{}],
        },
        "gateways": [{}],
        "describedNodes":[{}, {}, {}, {}]
    }

    base_dir = args[0]
    base_network = add_mixnode(base_network, base_dir, 1)
    base_network = add_mixnode(base_network, base_dir, 2)
    base_network = add_mixnode(base_network, base_dir, 3)
    base_network = add_gateway(base_network, base_dir)

    with open(os.path.join(base_dir, "network.json"), "w") as out:
        json.dump(base_network, out, indent=2)


def gateway_template():
    return {"Gateway": {
            "bond": {
                "pledge_amount": {
                    "denom": "unym",
                    "amount": "0"
                },
                "owner": "whatever",
                "block_height": 0,
                "gateway": {
                    "host": "TO_BE_FILLED",
                    "mix_port": "TO_BE_FILLED",
                    "clients_port": "TO_BE_FILLED",
                    "location": "whatever",
                    "sphinx_key": "TO_BE_FILLED",
                    "identity_key": "TO_BE_FILLED",
                    "version": "whatever",
                },
                "proxy": None,
            },
            "self_described": {
                "host_information": {
                    "ip_address": [
                        "0.0.0.0"
                    ],
                    "hostname": None,
                    "keys": {
                        "ed25519": "TO_BE_FILLED",
                        "x25519": "TO_BE_FILLED"
                    }
                },
                "build_information": {
                    "binary_name": "whatever",
                    "build_timestamp": "whatever",
                    "build_version": "whatever",
                    "commit_sha": "whatever",
                    "commit_timestamp": "whatever",
                    "commit_branch": "whatever",
                    "rustc_version": "whatever",
                    "rustc_channel": "whatever",
                    "cargo_profile": "whatever"
                },
                "network_requester": {
                    "address": "none",
                    "uses_exit_policy": True
                },
                "mixnet_websockets": {
                    "ws_port": 9000,
                    "wss_port": None
                },
                "noise_information": {
                    "supported": True
                }
            }
        }}

def mixnode_template():
    return {
        "Mixnode": {
            "bond": {
                "mix_id": "TO_BE_FILLED",
                "owner": "whatever",
                "original_pledge": {
                    "denom": "unym",
                    "amount": "0"
                },
                "layer": "TO_BE_FILLED",
                "mix_node": {
                    "host": "TO_BE_FILLED",
                    "mix_port": "TO_BE_FILLED",
                    "verloc_port": 1790,
                    "http_api_port": 8000,
                    "sphinx_key": "TO_BE_FILLED",
                    "identity_key": "TO_BE_FILLED",
                    "version": "whatever"
                },
                "proxy": None,
                "bonding_height": 0,
                "is_unbonding": False
            },
            "self_described": {
                "host_information": {
                    "ip_address": [
                        "0.0.0.0"
                    ],
                    "hostname": None,
                    "keys": {
                        "ed25519": "TO_BE_FILLED",
                        "x25519": "TO_BE_FILLED"
                    }
                },
                "build_information": {
                    "binary_name": "whatever",
                    "build_timestamp": "whatever",
                    "build_version": "whatever",
                    "commit_sha": "whatever",
                    "commit_timestamp": "whatever",
                    "commit_branch": "whatever",
                    "rustc_version": "whatever",
                    "rustc_channel": "whatever",
                    "cargo_profile": "whatever"
                },
                "network_requester": None,
                "ip_packet_router": None,
                "mixnet_websockets": None,
                "noise_information": {
                    "supported": True
                }
            }
        }
    }

if __name__ == '__main__':
    main(sys.argv[1:])
