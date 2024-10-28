```sh
[2m2024-10-28T15:01:31.692961Z[0m [32m INFO[0m [2mnym-api/src/main.rs[0m[2m:[0m[2m41[0m[2m:[0m Starting nym api...
Initialise a Nym Api instance with persistent config.toml file

Usage: nym-api init [OPTIONS]

Options:
      --id <ID>
          Id of the nym-api we want to initialise. if unspecified, a default value will be used. default: "default" [env: NYMAPI_ID_ARG=] [default: default]
  -m, --enable-monitor
          Specifies whether network monitoring is enabled on this API default: false [env: NYMAPI_ENABLE_MONITOR_ARG=]
  -r, --enable-rewarding
          Specifies whether network rewarding is enabled on this API default: false [env: NYMAPI_ENABLE_REWARDING_ARG=]
      --nyxd-validator <NYXD_VALIDATOR>
          Endpoint to nyxd instance used for contract information. default: http://localhost:26657 [env: NYMAPI_NYXD_VALIDATOR_ARG=]
      --mnemonic <MNEMONIC>
          Mnemonic of the network monitor used for sending rewarding and zk-nyms transactions default: None [env: NYMAPI_MNEMONIC_ARG=]
      --enable-zk-nym
          Flag to indicate whether credential signer authority is enabled on this API default: false [env: NYMAPI_ENABLE_ZK_NYM_ARG=]
      --announce-address <ANNOUNCE_ADDRESS>
          Announced address that is going to be put in the DKG contract where zk-nym clients will connect to obtain their credentials default: None [env: NYMAPI_ANNOUNCE_ADDRESS_NYM_ARG=]
      --monitor-credentials-mode
          Set this nym api to work in a enabled credentials that would attempt to use gateway with the bandwidth credential requirement [env: NYMAPI_MONITOR_CREDENTIALS_MODE_ARG=]
      --bind-address <BIND_ADDRESS>
          Socket address this api will use for binding its http API. default: `127.0.0.1:8080` in `debug` builds and `0.0.0.0:8080` in `release`
  -h, --help
          Print help
```
