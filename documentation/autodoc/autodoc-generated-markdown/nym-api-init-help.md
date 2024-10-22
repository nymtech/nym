```sh
Initialise a Nym Api instance with persistent config.toml file

Usage: nym-api init [OPTIONS]

Options:
      --id <ID>
          Id of the nym-api we want to initialise. if unspecified, a default value will be used. default: "default" [default: default]
  -m, --enable-monitor
          Specifies whether network monitoring is enabled on this API default: false
  -r, --enable-rewarding
          Specifies whether network rewarding is enabled on this API default: false
      --nyxd-validator <NYXD_VALIDATOR>
          Endpoint to nyxd instance used for contract information. default: http://localhost:26657
      --mnemonic <MNEMONIC>
          Mnemonic of the network monitor used for sending rewarding and zk-nyms transactions default: None
      --enable-zk-nym
          Flag to indicate whether credential signer authority is enabled on this API default: false
      --announce-address <ANNOUNCE_ADDRESS>
          Announced address that is going to be put in the DKG contract where zk-nym clients will connect to obtain their credentials default: None
      --monitor-credentials-mode
          Set this nym api to work in a enabled credentials that would attempt to use gateway with the bandwidth credential requirement
  -h, --help
          Print help
```
