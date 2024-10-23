```sh
Usage: nym-node [OPTIONS] <COMMAND>

Commands:
  build-info           Show build information of this binary
  bonding-information  Show bonding information of this node depending on its currently selected mode
  node-details         Show details of this node
  migrate              Attempt to migrate an existing mixnode or gateway into a nym-node
  run                  Start this nym-node
  sign                 Use identity key of this node to sign provided message
  help                 Print this message or the help of the given subcommand(s)

Options:
  -c, --config-env-file <CONFIG_ENV_FILE>
          Path pointing to an env file that configures the nym-node and overrides any preconfigured values [env: NYMNODE_CONFIG_ENV_FILE_ARG=]
      --no-banner
          Flag used for disabling the printed banner in tty [env: NYMNODE_NO_BANNER=]
  -h, --help
          Print help
  -V, --version
          Print version
```
