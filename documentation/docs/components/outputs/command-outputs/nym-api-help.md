```sh
[2m2024-11-06T10:12:52.501648Z[0m [32m INFO[0m [2mnym-api/src/main.rs[0m[2m:[0m[2m41[0m[2m:[0m Starting nym api...
Usage: nym-api [OPTIONS] <COMMAND>

Commands:
  init        Initialise a Nym Api instance with persistent config.toml file
  run         Run the Nym Api with provided configuration optionally overriding set parameters
  build-info  Show build information of this binary
  help        Print this message or the help of the given subcommand(s)

Options:
  -c, --config-env-file <CONFIG_ENV_FILE>
          Path pointing to an env file that configures the Nym API [env: NYMAPI_CONFIG_ENV_FILE_ARG=]
      --no-banner
          A no-op flag included for consistency with other binaries (and compatibility with nymvisor, oops) [env: NYMAPI_NO_BANNER_ARG=]
  -h, --help
          Print help
  -V, --version
          Print version
```
