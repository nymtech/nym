```sh
Show bonding information of this node depending on its currently selected mode

Usage: nym-node bonding-information [OPTIONS]

Options:
      --id <ID>                    Id of the nym-node to use [env: NYMNODE_ID=] [default: default-nym-node]
      --config-file <CONFIG_FILE>  Path to a configuration file of this node [env: NYMNODE_CONFIG=]
      --mode <MODE>                [env: NYMNODE_MODE=] [possible values: mixnode, entry-gateway, exit-gateway]
  -o, --output <OUTPUT>            Specify the output format of the bonding information (`text` or `json`) [default: text] [possible values: text, json]
  -h, --help                       Print help
```
