```sh
Use identity key of this node to sign provided message

Usage: nym-node sign [OPTIONS] <--text <TEXT>|--contract-msg <CONTRACT_MSG>>

Options:
      --id <ID>                      Id of the nym-node to use [env: NYMNODE_ID=] [default: default-nym-node]
      --config-file <CONFIG_FILE>    Path to a configuration file of this node [env: NYMNODE_CONFIG=]
      --text <TEXT>                  Signs an arbitrary piece of text with your identity key
      --contract-msg <CONTRACT_MSG>  Signs a transaction-specific payload, that is going to be sent to the smart contract, with your identity key
  -o, --output <OUTPUT>              [default: text] [possible values: text, json]
  -h, --help                         Print help
```
