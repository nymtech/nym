```sh
Usage: nymvisor [OPTIONS] <COMMAND>

Commands:
  init               Initialise a nymvisor instance with persistent Config.toml file
  run                Run the associated daemon with the preconfigured settings
  build-info         Show build information of this binary
  daemon-build-info  Show build information of the associated daemon
  add-upgrade        Queues up another upgrade for the associated daemon
  config             Show configuration options being used by this instance of nymvisor
  help               Print this message or the help of the given subcommand(s)

Options:
  -c, --config-env-file <CONFIG_ENV_FILE>
          Path pointing to an env file that configures the nymvisor and overrides any preconfigured values
  -h, --help
          Print help
  -V, --version
          Print version
```
