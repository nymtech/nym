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
        return base_network


def main(args):
    base_network = {
        "mixnodes": {
            "1": [{}],
            "2": [{}],
            "3": [{}],
        },
        "gateways": [{}]
    }

    base_dir = args[0]
    base_network = add_mixnode(base_network, base_dir, 1)
    base_network = add_mixnode(base_network, base_dir, 2)
    base_network = add_mixnode(base_network, base_dir, 3)
    base_network = add_gateway(base_network, base_dir)

    with open(os.path.join(base_dir, "network.json"), "w") as out:
        json.dump(base_network, out, indent=2)


if __name__ == '__main__':
    main(sys.argv[1:])
