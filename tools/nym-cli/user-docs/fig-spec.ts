const completion: Fig.Spec = {
  name: "nym-cli",
  description: "A client for interacting with Nym smart contracts and the Nyx blockchain",
  subcommands: [
    {
      name: "account",
      description: "Query and manage Nyx blockchain accounts",
      subcommands: [
        {
          name: "create",
          description: "Create a new mnemonic - note, this account does not appear on the chain until the account id is used in a transaction",
          options: [
            {
              name: "--word-count",
              args: {
                name: "word-count",
                isOptional: true,
              },
            },
            {
              name: "--mnemonic",
              description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
              args: {
                name: "mnemonic",
                isOptional: true,
              },
            },
            {
              name: "--config-env-file",
              description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
              args: {
                name: "config-env-file",
                isOptional: true,
              },
            },
            {
              name: "--nyxd-url",
              description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
              args: {
                name: "nyxd-url",
                isOptional: true,
              },
            },
            {
              name: "--validator-api-url",
              description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
              args: {
                name: "validator-api-url",
                isOptional: true,
              },
            },
            {
              name: "--mixnet-contract-address",
              description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
              args: {
                name: "mixnet-contract-address",
                isOptional: true,
              },
            },
            {
              name: "--vesting-contract-address",
              description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
              args: {
                name: "vesting-contract-address",
                isOptional: true,
              },
            },
            {
              name: ["-h", "--help"],
              description: "Print help information",
            },
          ],
        },
        {
          name: "balance",
          description: "Gets the balance of an account",
          options: [
            {
              name: "--denom",
              description: "Optional currency to show balance for",
              args: {
                name: "denom",
                isOptional: true,
              },
            },
            {
              name: "--mnemonic",
              description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
              args: {
                name: "mnemonic",
                isOptional: true,
              },
            },
            {
              name: "--config-env-file",
              description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
              args: {
                name: "config-env-file",
                isOptional: true,
              },
            },
            {
              name: "--nyxd-url",
              description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
              args: {
                name: "nyxd-url",
                isOptional: true,
              },
            },
            {
              name: "--validator-api-url",
              description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
              args: {
                name: "validator-api-url",
                isOptional: true,
              },
            },
            {
              name: "--mixnet-contract-address",
              description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
              args: {
                name: "mixnet-contract-address",
                isOptional: true,
              },
            },
            {
              name: "--vesting-contract-address",
              description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
              args: {
                name: "vesting-contract-address",
                isOptional: true,
              },
            },
            {
              name: "--hide-denom",
              description: "Optionally hide the denom",
            },
            {
              name: "--raw",
              description: "Show as a raw value",
            },
            {
              name: ["-h", "--help"],
              description: "Print help information",
            },
          ],
          args: {
            name: "address",
            isOptional: true,
          },
        },
        {
          name: "pub-key",
          description: "Gets the public key of an account",
          options: [
            {
              name: "--mnemonic",
              description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
              args: {
                name: "mnemonic",
                isOptional: true,
              },
            },
            {
              name: "--config-env-file",
              description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
              args: {
                name: "config-env-file",
                isOptional: true,
              },
            },
            {
              name: "--nyxd-url",
              description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
              args: {
                name: "nyxd-url",
                isOptional: true,
              },
            },
            {
              name: "--validator-api-url",
              description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
              args: {
                name: "validator-api-url",
                isOptional: true,
              },
            },
            {
              name: "--mixnet-contract-address",
              description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
              args: {
                name: "mixnet-contract-address",
                isOptional: true,
              },
            },
            {
              name: "--vesting-contract-address",
              description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
              args: {
                name: "vesting-contract-address",
                isOptional: true,
              },
            },
            {
              name: "--from-mnemonic",
              description: "If set, get the public key from the mnemonic, rather than querying for it",
            },
            {
              name: ["-h", "--help"],
              description: "Print help information",
            },
          ],
          args: {
            name: "address",
            isOptional: true,
          },
        },
        {
          name: "send",
          description: "Sends tokens to another account",
          options: [
            {
              name: "--denom",
              description: "Override the denomination",
              args: {
                name: "denom",
                isOptional: true,
              },
            },
            {
              name: "--memo",
              args: {
                name: "memo",
                isOptional: true,
              },
            },
            {
              name: "--mnemonic",
              description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
              args: {
                name: "mnemonic",
                isOptional: true,
              },
            },
            {
              name: "--config-env-file",
              description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
              args: {
                name: "config-env-file",
                isOptional: true,
              },
            },
            {
              name: "--nyxd-url",
              description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
              args: {
                name: "nyxd-url",
                isOptional: true,
              },
            },
            {
              name: "--validator-api-url",
              description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
              args: {
                name: "validator-api-url",
                isOptional: true,
              },
            },
            {
              name: "--mixnet-contract-address",
              description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
              args: {
                name: "mixnet-contract-address",
                isOptional: true,
              },
            },
            {
              name: "--vesting-contract-address",
              description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
              args: {
                name: "vesting-contract-address",
                isOptional: true,
              },
            },
            {
              name: ["-h", "--help"],
              description: "Print help information",
            },
          ],
          args: [
            {
              name: "recipient",
            },
            {
              name: "amount",
            },
          ]
        },
        {
          name: "help",
          description: "Print this message or the help of the given subcommand(s)",
          options: [
            {
              name: "--mnemonic",
              description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
              args: {
                name: "mnemonic",
                isOptional: true,
              },
            },
            {
              name: "--config-env-file",
              description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
              args: {
                name: "config-env-file",
                isOptional: true,
              },
            },
            {
              name: "--nyxd-url",
              description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
              args: {
                name: "nyxd-url",
                isOptional: true,
              },
            },
            {
              name: "--validator-api-url",
              description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
              args: {
                name: "validator-api-url",
                isOptional: true,
              },
            },
            {
              name: "--mixnet-contract-address",
              description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
              args: {
                name: "mixnet-contract-address",
                isOptional: true,
              },
            },
            {
              name: "--vesting-contract-address",
              description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
              args: {
                name: "vesting-contract-address",
                isOptional: true,
              },
            },
          ],
          args: {
            name: "subcommand",
            isOptional: true,
          },
        },
      ],
      options: [
        {
          name: "--mnemonic",
          description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
          args: {
            name: "mnemonic",
            isOptional: true,
          },
        },
        {
          name: "--config-env-file",
          description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
          args: {
            name: "config-env-file",
            isOptional: true,
          },
        },
        {
          name: "--nyxd-url",
          description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
          args: {
            name: "nyxd-url",
            isOptional: true,
          },
        },
        {
          name: "--validator-api-url",
          description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
          args: {
            name: "validator-api-url",
            isOptional: true,
          },
        },
        {
          name: "--mixnet-contract-address",
          description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
          args: {
            name: "mixnet-contract-address",
            isOptional: true,
          },
        },
        {
          name: "--vesting-contract-address",
          description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
          args: {
            name: "vesting-contract-address",
            isOptional: true,
          },
        },
        {
          name: ["-h", "--help"],
          description: "Print help information",
        },
      ],
    },
    {
      name: "signature",
      description: "Sign and verify messages",
      subcommands: [
        {
          name: "sign",
          description: "Sign a message",
          options: [
            {
              name: "--mnemonic",
              description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
              args: {
                name: "mnemonic",
                isOptional: true,
              },
            },
            {
              name: "--config-env-file",
              description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
              args: {
                name: "config-env-file",
                isOptional: true,
              },
            },
            {
              name: "--nyxd-url",
              description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
              args: {
                name: "nyxd-url",
                isOptional: true,
              },
            },
            {
              name: "--validator-api-url",
              description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
              args: {
                name: "validator-api-url",
                isOptional: true,
              },
            },
            {
              name: "--mixnet-contract-address",
              description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
              args: {
                name: "mixnet-contract-address",
                isOptional: true,
              },
            },
            {
              name: "--vesting-contract-address",
              description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
              args: {
                name: "vesting-contract-address",
                isOptional: true,
              },
            },
            {
              name: ["-h", "--help"],
              description: "Print help information",
            },
          ],
          args: {
            name: "message",
          },
        },
        {
          name: "verify",
          description: "Verify a message",
          options: [
            {
              name: "--mnemonic",
              description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
              args: {
                name: "mnemonic",
                isOptional: true,
              },
            },
            {
              name: "--config-env-file",
              description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
              args: {
                name: "config-env-file",
                isOptional: true,
              },
            },
            {
              name: "--nyxd-url",
              description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
              args: {
                name: "nyxd-url",
                isOptional: true,
              },
            },
            {
              name: "--validator-api-url",
              description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
              args: {
                name: "validator-api-url",
                isOptional: true,
              },
            },
            {
              name: "--mixnet-contract-address",
              description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
              args: {
                name: "mixnet-contract-address",
                isOptional: true,
              },
            },
            {
              name: "--vesting-contract-address",
              description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
              args: {
                name: "vesting-contract-address",
                isOptional: true,
              },
            },
            {
              name: ["-h", "--help"],
              description: "Print help information",
            },
          ],
          args: [
            {
              name: "public-key-or-address",
            },
            {
              name: "signature-as-hex",
            },
            {
              name: "message",
            },
          ]
        },
        {
          name: "help",
          description: "Print this message or the help of the given subcommand(s)",
          options: [
            {
              name: "--mnemonic",
              description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
              args: {
                name: "mnemonic",
                isOptional: true,
              },
            },
            {
              name: "--config-env-file",
              description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
              args: {
                name: "config-env-file",
                isOptional: true,
              },
            },
            {
              name: "--nyxd-url",
              description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
              args: {
                name: "nyxd-url",
                isOptional: true,
              },
            },
            {
              name: "--validator-api-url",
              description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
              args: {
                name: "validator-api-url",
                isOptional: true,
              },
            },
            {
              name: "--mixnet-contract-address",
              description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
              args: {
                name: "mixnet-contract-address",
                isOptional: true,
              },
            },
            {
              name: "--vesting-contract-address",
              description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
              args: {
                name: "vesting-contract-address",
                isOptional: true,
              },
            },
          ],
          args: {
            name: "subcommand",
            isOptional: true,
          },
        },
      ],
      options: [
        {
          name: "--mnemonic",
          description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
          args: {
            name: "mnemonic",
            isOptional: true,
          },
        },
        {
          name: "--config-env-file",
          description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
          args: {
            name: "config-env-file",
            isOptional: true,
          },
        },
        {
          name: "--nyxd-url",
          description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
          args: {
            name: "nyxd-url",
            isOptional: true,
          },
        },
        {
          name: "--validator-api-url",
          description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
          args: {
            name: "validator-api-url",
            isOptional: true,
          },
        },
        {
          name: "--mixnet-contract-address",
          description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
          args: {
            name: "mixnet-contract-address",
            isOptional: true,
          },
        },
        {
          name: "--vesting-contract-address",
          description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
          args: {
            name: "vesting-contract-address",
            isOptional: true,
          },
        },
        {
          name: ["-h", "--help"],
          description: "Print help information",
        },
      ],
    },
    {
      name: "block",
      description: "Query chain blocks",
      subcommands: [
        {
          name: "get",
          description: "Gets a block's details and prints as JSON",
          options: [
            {
              name: "--mnemonic",
              description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
              args: {
                name: "mnemonic",
                isOptional: true,
              },
            },
            {
              name: "--config-env-file",
              description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
              args: {
                name: "config-env-file",
                isOptional: true,
              },
            },
            {
              name: "--nyxd-url",
              description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
              args: {
                name: "nyxd-url",
                isOptional: true,
              },
            },
            {
              name: "--validator-api-url",
              description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
              args: {
                name: "validator-api-url",
                isOptional: true,
              },
            },
            {
              name: "--mixnet-contract-address",
              description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
              args: {
                name: "mixnet-contract-address",
                isOptional: true,
              },
            },
            {
              name: "--vesting-contract-address",
              description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
              args: {
                name: "vesting-contract-address",
                isOptional: true,
              },
            },
            {
              name: ["-h", "--help"],
              description: "Print help information",
            },
          ],
          args: {
            name: "height",
          },
        },
        {
          name: "time",
          description: "Gets the block time at a height",
          options: [
            {
              name: "--mnemonic",
              description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
              args: {
                name: "mnemonic",
                isOptional: true,
              },
            },
            {
              name: "--config-env-file",
              description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
              args: {
                name: "config-env-file",
                isOptional: true,
              },
            },
            {
              name: "--nyxd-url",
              description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
              args: {
                name: "nyxd-url",
                isOptional: true,
              },
            },
            {
              name: "--validator-api-url",
              description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
              args: {
                name: "validator-api-url",
                isOptional: true,
              },
            },
            {
              name: "--mixnet-contract-address",
              description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
              args: {
                name: "mixnet-contract-address",
                isOptional: true,
              },
            },
            {
              name: "--vesting-contract-address",
              description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
              args: {
                name: "vesting-contract-address",
                isOptional: true,
              },
            },
            {
              name: ["-h", "--help"],
              description: "Print help information",
            },
          ],
          args: {
            name: "height",
          },
        },
        {
          name: "current-height",
          description: "Gets the current block height",
          options: [
            {
              name: "--mnemonic",
              description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
              args: {
                name: "mnemonic",
                isOptional: true,
              },
            },
            {
              name: "--config-env-file",
              description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
              args: {
                name: "config-env-file",
                isOptional: true,
              },
            },
            {
              name: "--nyxd-url",
              description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
              args: {
                name: "nyxd-url",
                isOptional: true,
              },
            },
            {
              name: "--validator-api-url",
              description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
              args: {
                name: "validator-api-url",
                isOptional: true,
              },
            },
            {
              name: "--mixnet-contract-address",
              description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
              args: {
                name: "mixnet-contract-address",
                isOptional: true,
              },
            },
            {
              name: "--vesting-contract-address",
              description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
              args: {
                name: "vesting-contract-address",
                isOptional: true,
              },
            },
            {
              name: ["-h", "--help"],
              description: "Print help information",
            },
          ],
        },
        {
          name: "help",
          description: "Print this message or the help of the given subcommand(s)",
          options: [
            {
              name: "--mnemonic",
              description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
              args: {
                name: "mnemonic",
                isOptional: true,
              },
            },
            {
              name: "--config-env-file",
              description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
              args: {
                name: "config-env-file",
                isOptional: true,
              },
            },
            {
              name: "--nyxd-url",
              description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
              args: {
                name: "nyxd-url",
                isOptional: true,
              },
            },
            {
              name: "--validator-api-url",
              description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
              args: {
                name: "validator-api-url",
                isOptional: true,
              },
            },
            {
              name: "--mixnet-contract-address",
              description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
              args: {
                name: "mixnet-contract-address",
                isOptional: true,
              },
            },
            {
              name: "--vesting-contract-address",
              description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
              args: {
                name: "vesting-contract-address",
                isOptional: true,
              },
            },
          ],
          args: {
            name: "subcommand",
            isOptional: true,
          },
        },
      ],
      options: [
        {
          name: "--mnemonic",
          description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
          args: {
            name: "mnemonic",
            isOptional: true,
          },
        },
        {
          name: "--config-env-file",
          description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
          args: {
            name: "config-env-file",
            isOptional: true,
          },
        },
        {
          name: "--nyxd-url",
          description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
          args: {
            name: "nyxd-url",
            isOptional: true,
          },
        },
        {
          name: "--validator-api-url",
          description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
          args: {
            name: "validator-api-url",
            isOptional: true,
          },
        },
        {
          name: "--mixnet-contract-address",
          description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
          args: {
            name: "mixnet-contract-address",
            isOptional: true,
          },
        },
        {
          name: "--vesting-contract-address",
          description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
          args: {
            name: "vesting-contract-address",
            isOptional: true,
          },
        },
        {
          name: ["-h", "--help"],
          description: "Print help information",
        },
      ],
    },
    {
      name: "cosmwasm",
      description: "Manage and execute WASM smart contracts",
      subcommands: [
        {
          name: "upload",
          description: "Upload a smart contract WASM blob",
          options: [
            {
              name: "--wasm-path",
              args: {
                name: "wasm-path",
              },
            },
            {
              name: "--memo",
              args: {
                name: "memo",
                isOptional: true,
              },
            },
            {
              name: "--mnemonic",
              description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
              args: {
                name: "mnemonic",
                isOptional: true,
              },
            },
            {
              name: "--config-env-file",
              description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
              args: {
                name: "config-env-file",
                isOptional: true,
              },
            },
            {
              name: "--nyxd-url",
              description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
              args: {
                name: "nyxd-url",
                isOptional: true,
              },
            },
            {
              name: "--validator-api-url",
              description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
              args: {
                name: "validator-api-url",
                isOptional: true,
              },
            },
            {
              name: "--mixnet-contract-address",
              description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
              args: {
                name: "mixnet-contract-address",
                isOptional: true,
              },
            },
            {
              name: "--vesting-contract-address",
              description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
              args: {
                name: "vesting-contract-address",
                isOptional: true,
              },
            },
            {
              name: ["-h", "--help"],
              description: "Print help information",
            },
          ],
        },
        {
          name: "init",
          description: "Init a WASM smart contract",
          options: [
            {
              name: "--memo",
              args: {
                name: "memo",
                isOptional: true,
              },
            },
            {
              name: "--label",
              args: {
                name: "label",
                isOptional: true,
              },
            },
            {
              name: "--init-message",
              args: {
                name: "init-message",
              },
            },
            {
              name: "--admin",
              args: {
                name: "admin",
                isOptional: true,
              },
            },
            {
              name: "--funds",
              description: "Amount to supply as funds in micro denomination (e.g. unym or unyx)",
              args: {
                name: "funds",
                isOptional: true,
              },
            },
            {
              name: "--funds-denom",
              description: "Set the denomination for the funds",
              args: {
                name: "funds-denom",
                isOptional: true,
              },
            },
            {
              name: "--mnemonic",
              description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
              args: {
                name: "mnemonic",
                isOptional: true,
              },
            },
            {
              name: "--config-env-file",
              description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
              args: {
                name: "config-env-file",
                isOptional: true,
              },
            },
            {
              name: "--nyxd-url",
              description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
              args: {
                name: "nyxd-url",
                isOptional: true,
              },
            },
            {
              name: "--validator-api-url",
              description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
              args: {
                name: "validator-api-url",
                isOptional: true,
              },
            },
            {
              name: "--mixnet-contract-address",
              description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
              args: {
                name: "mixnet-contract-address",
                isOptional: true,
              },
            },
            {
              name: "--vesting-contract-address",
              description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
              args: {
                name: "vesting-contract-address",
                isOptional: true,
              },
            },
            {
              name: ["-h", "--help"],
              description: "Print help information",
            },
          ],
          args: {
            name: "code-id",
          },
        },
        {
          name: "migrate",
          description: "Migrate a WASM smart contract",
          options: [
            {
              name: "--code-id",
              args: {
                name: "code-id",
              },
            },
            {
              name: "--memo",
              args: {
                name: "memo",
                isOptional: true,
              },
            },
            {
              name: "--init-message",
              args: {
                name: "init-message",
                isOptional: true,
              },
            },
            {
              name: "--mnemonic",
              description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
              args: {
                name: "mnemonic",
                isOptional: true,
              },
            },
            {
              name: "--config-env-file",
              description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
              args: {
                name: "config-env-file",
                isOptional: true,
              },
            },
            {
              name: "--nyxd-url",
              description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
              args: {
                name: "nyxd-url",
                isOptional: true,
              },
            },
            {
              name: "--validator-api-url",
              description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
              args: {
                name: "validator-api-url",
                isOptional: true,
              },
            },
            {
              name: "--mixnet-contract-address",
              description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
              args: {
                name: "mixnet-contract-address",
                isOptional: true,
              },
            },
            {
              name: "--vesting-contract-address",
              description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
              args: {
                name: "vesting-contract-address",
                isOptional: true,
              },
            },
            {
              name: ["-h", "--help"],
              description: "Print help information",
            },
          ],
          args: {
            name: "contract-address",
          },
        },
        {
          name: "execute",
          description: "Execute a WASM smart contract method",
          options: [
            {
              name: "--memo",
              args: {
                name: "memo",
                isOptional: true,
              },
            },
            {
              name: "--funds-denom",
              description: "Set the denomination for the funds",
              args: {
                name: "funds-denom",
                isOptional: true,
              },
            },
            {
              name: "--mnemonic",
              description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
              args: {
                name: "mnemonic",
                isOptional: true,
              },
            },
            {
              name: "--config-env-file",
              description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
              args: {
                name: "config-env-file",
                isOptional: true,
              },
            },
            {
              name: "--nyxd-url",
              description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
              args: {
                name: "nyxd-url",
                isOptional: true,
              },
            },
            {
              name: "--validator-api-url",
              description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
              args: {
                name: "validator-api-url",
                isOptional: true,
              },
            },
            {
              name: "--mixnet-contract-address",
              description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
              args: {
                name: "mixnet-contract-address",
                isOptional: true,
              },
            },
            {
              name: "--vesting-contract-address",
              description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
              args: {
                name: "vesting-contract-address",
                isOptional: true,
              },
            },
            {
              name: ["-h", "--help"],
              description: "Print help information",
            },
          ],
          args: [
            {
              name: "contract-address",
            },
            {
              name: "json-args",
            },
            {
              name: "funds",
              isOptional: true,
            },
          ]
        },
        {
          name: "help",
          description: "Print this message or the help of the given subcommand(s)",
          options: [
            {
              name: "--mnemonic",
              description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
              args: {
                name: "mnemonic",
                isOptional: true,
              },
            },
            {
              name: "--config-env-file",
              description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
              args: {
                name: "config-env-file",
                isOptional: true,
              },
            },
            {
              name: "--nyxd-url",
              description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
              args: {
                name: "nyxd-url",
                isOptional: true,
              },
            },
            {
              name: "--validator-api-url",
              description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
              args: {
                name: "validator-api-url",
                isOptional: true,
              },
            },
            {
              name: "--mixnet-contract-address",
              description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
              args: {
                name: "mixnet-contract-address",
                isOptional: true,
              },
            },
            {
              name: "--vesting-contract-address",
              description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
              args: {
                name: "vesting-contract-address",
                isOptional: true,
              },
            },
          ],
          args: {
            name: "subcommand",
            isOptional: true,
          },
        },
      ],
      options: [
        {
          name: "--mnemonic",
          description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
          args: {
            name: "mnemonic",
            isOptional: true,
          },
        },
        {
          name: "--config-env-file",
          description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
          args: {
            name: "config-env-file",
            isOptional: true,
          },
        },
        {
          name: "--nyxd-url",
          description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
          args: {
            name: "nyxd-url",
            isOptional: true,
          },
        },
        {
          name: "--validator-api-url",
          description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
          args: {
            name: "validator-api-url",
            isOptional: true,
          },
        },
        {
          name: "--mixnet-contract-address",
          description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
          args: {
            name: "mixnet-contract-address",
            isOptional: true,
          },
        },
        {
          name: "--vesting-contract-address",
          description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
          args: {
            name: "vesting-contract-address",
            isOptional: true,
          },
        },
        {
          name: ["-h", "--help"],
          description: "Print help information",
        },
      ],
    },
    {
      name: "tx",
      description: "Query for transactions",
      subcommands: [
        {
          name: "get",
          description: "Get a transaction by hash or block height",
          options: [
            {
              name: "--mnemonic",
              description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
              args: {
                name: "mnemonic",
                isOptional: true,
              },
            },
            {
              name: "--config-env-file",
              description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
              args: {
                name: "config-env-file",
                isOptional: true,
              },
            },
            {
              name: "--nyxd-url",
              description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
              args: {
                name: "nyxd-url",
                isOptional: true,
              },
            },
            {
              name: "--validator-api-url",
              description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
              args: {
                name: "validator-api-url",
                isOptional: true,
              },
            },
            {
              name: "--mixnet-contract-address",
              description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
              args: {
                name: "mixnet-contract-address",
                isOptional: true,
              },
            },
            {
              name: "--vesting-contract-address",
              description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
              args: {
                name: "vesting-contract-address",
                isOptional: true,
              },
            },
            {
              name: ["-h", "--help"],
              description: "Print help information",
            },
          ],
          args: {
            name: "tx-hash",
          },
        },
        {
          name: "query",
          description: "Query for transactions",
          options: [
            {
              name: "--mnemonic",
              description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
              args: {
                name: "mnemonic",
                isOptional: true,
              },
            },
            {
              name: "--config-env-file",
              description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
              args: {
                name: "config-env-file",
                isOptional: true,
              },
            },
            {
              name: "--nyxd-url",
              description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
              args: {
                name: "nyxd-url",
                isOptional: true,
              },
            },
            {
              name: "--validator-api-url",
              description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
              args: {
                name: "validator-api-url",
                isOptional: true,
              },
            },
            {
              name: "--mixnet-contract-address",
              description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
              args: {
                name: "mixnet-contract-address",
                isOptional: true,
              },
            },
            {
              name: "--vesting-contract-address",
              description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
              args: {
                name: "vesting-contract-address",
                isOptional: true,
              },
            },
            {
              name: ["-h", "--help"],
              description: "Print help information",
            },
          ],
          args: {
            name: "query",
          },
        },
        {
          name: "help",
          description: "Print this message or the help of the given subcommand(s)",
          options: [
            {
              name: "--mnemonic",
              description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
              args: {
                name: "mnemonic",
                isOptional: true,
              },
            },
            {
              name: "--config-env-file",
              description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
              args: {
                name: "config-env-file",
                isOptional: true,
              },
            },
            {
              name: "--nyxd-url",
              description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
              args: {
                name: "nyxd-url",
                isOptional: true,
              },
            },
            {
              name: "--validator-api-url",
              description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
              args: {
                name: "validator-api-url",
                isOptional: true,
              },
            },
            {
              name: "--mixnet-contract-address",
              description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
              args: {
                name: "mixnet-contract-address",
                isOptional: true,
              },
            },
            {
              name: "--vesting-contract-address",
              description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
              args: {
                name: "vesting-contract-address",
                isOptional: true,
              },
            },
          ],
          args: {
            name: "subcommand",
            isOptional: true,
          },
        },
      ],
      options: [
        {
          name: "--mnemonic",
          description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
          args: {
            name: "mnemonic",
            isOptional: true,
          },
        },
        {
          name: "--config-env-file",
          description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
          args: {
            name: "config-env-file",
            isOptional: true,
          },
        },
        {
          name: "--nyxd-url",
          description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
          args: {
            name: "nyxd-url",
            isOptional: true,
          },
        },
        {
          name: "--validator-api-url",
          description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
          args: {
            name: "validator-api-url",
            isOptional: true,
          },
        },
        {
          name: "--mixnet-contract-address",
          description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
          args: {
            name: "mixnet-contract-address",
            isOptional: true,
          },
        },
        {
          name: "--vesting-contract-address",
          description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
          args: {
            name: "vesting-contract-address",
            isOptional: true,
          },
        },
        {
          name: ["-h", "--help"],
          description: "Print help information",
        },
      ],
    },
    {
      name: "vesting-schedule",
      description: "Create and query for a vesting schedule",
      subcommands: [
        {
          name: "create",
          description: "Creates a vesting schedule",
          options: [
            {
              name: "--periods-seconds",
              args: {
                name: "periods-seconds",
                isOptional: true,
              },
            },
            {
              name: "--number-of-periods",
              args: {
                name: "number-of-periods",
                isOptional: true,
              },
            },
            {
              name: "--start-time",
              args: {
                name: "start-time",
                isOptional: true,
              },
            },
            {
              name: "--address",
              args: {
                name: "address",
              },
            },
            {
              name: "--amount",
              args: {
                name: "amount",
              },
            },
            {
              name: "--staking-address",
              args: {
                name: "staking-address",
                isOptional: true,
              },
            },
            {
              name: "--mnemonic",
              description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
              args: {
                name: "mnemonic",
                isOptional: true,
              },
            },
            {
              name: "--config-env-file",
              description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
              args: {
                name: "config-env-file",
                isOptional: true,
              },
            },
            {
              name: "--nyxd-url",
              description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
              args: {
                name: "nyxd-url",
                isOptional: true,
              },
            },
            {
              name: "--validator-api-url",
              description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
              args: {
                name: "validator-api-url",
                isOptional: true,
              },
            },
            {
              name: "--mixnet-contract-address",
              description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
              args: {
                name: "mixnet-contract-address",
                isOptional: true,
              },
            },
            {
              name: "--vesting-contract-address",
              description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
              args: {
                name: "vesting-contract-address",
                isOptional: true,
              },
            },
            {
              name: ["-h", "--help"],
              description: "Print help information",
            },
          ],
        },
        {
          name: "query",
          description: "Query for vesting schedule",
          options: [
            {
              name: "--mnemonic",
              description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
              args: {
                name: "mnemonic",
                isOptional: true,
              },
            },
            {
              name: "--config-env-file",
              description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
              args: {
                name: "config-env-file",
                isOptional: true,
              },
            },
            {
              name: "--nyxd-url",
              description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
              args: {
                name: "nyxd-url",
                isOptional: true,
              },
            },
            {
              name: "--validator-api-url",
              description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
              args: {
                name: "validator-api-url",
                isOptional: true,
              },
            },
            {
              name: "--mixnet-contract-address",
              description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
              args: {
                name: "mixnet-contract-address",
                isOptional: true,
              },
            },
            {
              name: "--vesting-contract-address",
              description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
              args: {
                name: "vesting-contract-address",
                isOptional: true,
              },
            },
            {
              name: ["-h", "--help"],
              description: "Print help information",
            },
          ],
          args: {
            name: "address",
            isOptional: true,
          },
        },
        {
          name: "vested-balance",
          description: "Get the amount that has vested and is free for withdrawal, delegation or bonding",
          options: [
            {
              name: "--mnemonic",
              description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
              args: {
                name: "mnemonic",
                isOptional: true,
              },
            },
            {
              name: "--config-env-file",
              description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
              args: {
                name: "config-env-file",
                isOptional: true,
              },
            },
            {
              name: "--nyxd-url",
              description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
              args: {
                name: "nyxd-url",
                isOptional: true,
              },
            },
            {
              name: "--validator-api-url",
              description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
              args: {
                name: "validator-api-url",
                isOptional: true,
              },
            },
            {
              name: "--mixnet-contract-address",
              description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
              args: {
                name: "mixnet-contract-address",
                isOptional: true,
              },
            },
            {
              name: "--vesting-contract-address",
              description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
              args: {
                name: "vesting-contract-address",
                isOptional: true,
              },
            },
            {
              name: ["-h", "--help"],
              description: "Print help information",
            },
          ],
          args: {
            name: "address",
            isOptional: true,
          },
        },
        {
          name: "withdraw-vested",
          description: "Withdraw vested tokens (note: the available amount excludes anything delegated or bonded before or after vesting)",
          options: [
            {
              name: "--mnemonic",
              description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
              args: {
                name: "mnemonic",
                isOptional: true,
              },
            },
            {
              name: "--config-env-file",
              description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
              args: {
                name: "config-env-file",
                isOptional: true,
              },
            },
            {
              name: "--nyxd-url",
              description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
              args: {
                name: "nyxd-url",
                isOptional: true,
              },
            },
            {
              name: "--validator-api-url",
              description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
              args: {
                name: "validator-api-url",
                isOptional: true,
              },
            },
            {
              name: "--mixnet-contract-address",
              description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
              args: {
                name: "mixnet-contract-address",
                isOptional: true,
              },
            },
            {
              name: "--vesting-contract-address",
              description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
              args: {
                name: "vesting-contract-address",
                isOptional: true,
              },
            },
            {
              name: ["-h", "--help"],
              description: "Print help information",
            },
          ],
          args: {
            name: "amount",
          },
        },
        {
          name: "help",
          description: "Print this message or the help of the given subcommand(s)",
          options: [
            {
              name: "--mnemonic",
              description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
              args: {
                name: "mnemonic",
                isOptional: true,
              },
            },
            {
              name: "--config-env-file",
              description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
              args: {
                name: "config-env-file",
                isOptional: true,
              },
            },
            {
              name: "--nyxd-url",
              description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
              args: {
                name: "nyxd-url",
                isOptional: true,
              },
            },
            {
              name: "--validator-api-url",
              description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
              args: {
                name: "validator-api-url",
                isOptional: true,
              },
            },
            {
              name: "--mixnet-contract-address",
              description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
              args: {
                name: "mixnet-contract-address",
                isOptional: true,
              },
            },
            {
              name: "--vesting-contract-address",
              description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
              args: {
                name: "vesting-contract-address",
                isOptional: true,
              },
            },
          ],
          args: {
            name: "subcommand",
            isOptional: true,
          },
        },
      ],
      options: [
        {
          name: "--mnemonic",
          description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
          args: {
            name: "mnemonic",
            isOptional: true,
          },
        },
        {
          name: "--config-env-file",
          description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
          args: {
            name: "config-env-file",
            isOptional: true,
          },
        },
        {
          name: "--nyxd-url",
          description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
          args: {
            name: "nyxd-url",
            isOptional: true,
          },
        },
        {
          name: "--validator-api-url",
          description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
          args: {
            name: "validator-api-url",
            isOptional: true,
          },
        },
        {
          name: "--mixnet-contract-address",
          description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
          args: {
            name: "mixnet-contract-address",
            isOptional: true,
          },
        },
        {
          name: "--vesting-contract-address",
          description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
          args: {
            name: "vesting-contract-address",
            isOptional: true,
          },
        },
        {
          name: ["-h", "--help"],
          description: "Print help information",
        },
      ],
    },
    {
      name: "mixnet",
      description: "Manage your mixnet infrastructure, delegate stake or query the directory",
      subcommands: [
        {
          name: "query",
          description: "Query the mixnet directory",
          subcommands: [
            {
              name: "mixnodes",
              description: "Query mixnodes",
              options: [
                {
                  name: "--mnemonic",
                  description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
                  args: {
                    name: "mnemonic",
                    isOptional: true,
                  },
                },
                {
                  name: "--config-env-file",
                  description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
                  args: {
                    name: "config-env-file",
                    isOptional: true,
                  },
                },
                {
                  name: "--nyxd-url",
                  description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
                  args: {
                    name: "nyxd-url",
                    isOptional: true,
                  },
                },
                {
                  name: "--validator-api-url",
                  description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
                  args: {
                    name: "validator-api-url",
                    isOptional: true,
                  },
                },
                {
                  name: "--mixnet-contract-address",
                  description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
                  args: {
                    name: "mixnet-contract-address",
                    isOptional: true,
                  },
                },
                {
                  name: "--vesting-contract-address",
                  description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
                  args: {
                    name: "vesting-contract-address",
                    isOptional: true,
                  },
                },
                {
                  name: ["-h", "--help"],
                  description: "Print help information",
                },
              ],
              args: {
                name: "identity-key",
                isOptional: true,
              },
            },
            {
              name: "gateways",
              description: "Query gateways",
              options: [
                {
                  name: "--mnemonic",
                  description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
                  args: {
                    name: "mnemonic",
                    isOptional: true,
                  },
                },
                {
                  name: "--config-env-file",
                  description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
                  args: {
                    name: "config-env-file",
                    isOptional: true,
                  },
                },
                {
                  name: "--nyxd-url",
                  description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
                  args: {
                    name: "nyxd-url",
                    isOptional: true,
                  },
                },
                {
                  name: "--validator-api-url",
                  description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
                  args: {
                    name: "validator-api-url",
                    isOptional: true,
                  },
                },
                {
                  name: "--mixnet-contract-address",
                  description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
                  args: {
                    name: "mixnet-contract-address",
                    isOptional: true,
                  },
                },
                {
                  name: "--vesting-contract-address",
                  description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
                  args: {
                    name: "vesting-contract-address",
                    isOptional: true,
                  },
                },
                {
                  name: ["-h", "--help"],
                  description: "Print help information",
                },
              ],
              args: {
                name: "identity-key",
                isOptional: true,
              },
            },
            {
              name: "help",
              description: "Print this message or the help of the given subcommand(s)",
              options: [
                {
                  name: "--mnemonic",
                  description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
                  args: {
                    name: "mnemonic",
                    isOptional: true,
                  },
                },
                {
                  name: "--config-env-file",
                  description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
                  args: {
                    name: "config-env-file",
                    isOptional: true,
                  },
                },
                {
                  name: "--nyxd-url",
                  description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
                  args: {
                    name: "nyxd-url",
                    isOptional: true,
                  },
                },
                {
                  name: "--validator-api-url",
                  description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
                  args: {
                    name: "validator-api-url",
                    isOptional: true,
                  },
                },
                {
                  name: "--mixnet-contract-address",
                  description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
                  args: {
                    name: "mixnet-contract-address",
                    isOptional: true,
                  },
                },
                {
                  name: "--vesting-contract-address",
                  description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
                  args: {
                    name: "vesting-contract-address",
                    isOptional: true,
                  },
                },
              ],
              args: {
                name: "subcommand",
                isOptional: true,
              },
            },
          ],
          options: [
            {
              name: "--mnemonic",
              description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
              args: {
                name: "mnemonic",
                isOptional: true,
              },
            },
            {
              name: "--config-env-file",
              description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
              args: {
                name: "config-env-file",
                isOptional: true,
              },
            },
            {
              name: "--nyxd-url",
              description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
              args: {
                name: "nyxd-url",
                isOptional: true,
              },
            },
            {
              name: "--validator-api-url",
              description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
              args: {
                name: "validator-api-url",
                isOptional: true,
              },
            },
            {
              name: "--mixnet-contract-address",
              description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
              args: {
                name: "mixnet-contract-address",
                isOptional: true,
              },
            },
            {
              name: "--vesting-contract-address",
              description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
              args: {
                name: "vesting-contract-address",
                isOptional: true,
              },
            },
            {
              name: ["-h", "--help"],
              description: "Print help information",
            },
          ],
        },
        {
          name: "delegators",
          description: "Manage your delegations",
          subcommands: [
            {
              name: "list",
              description: "Lists current delegations",
              options: [
                {
                  name: "--mnemonic",
                  description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
                  args: {
                    name: "mnemonic",
                    isOptional: true,
                  },
                },
                {
                  name: "--config-env-file",
                  description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
                  args: {
                    name: "config-env-file",
                    isOptional: true,
                  },
                },
                {
                  name: "--nyxd-url",
                  description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
                  args: {
                    name: "nyxd-url",
                    isOptional: true,
                  },
                },
                {
                  name: "--validator-api-url",
                  description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
                  args: {
                    name: "validator-api-url",
                    isOptional: true,
                  },
                },
                {
                  name: "--mixnet-contract-address",
                  description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
                  args: {
                    name: "mixnet-contract-address",
                    isOptional: true,
                  },
                },
                {
                  name: "--vesting-contract-address",
                  description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
                  args: {
                    name: "vesting-contract-address",
                    isOptional: true,
                  },
                },
                {
                  name: ["-h", "--help"],
                  description: "Print help information",
                },
              ],
            },
            {
              name: "rewards",
              description: "Manage rewards from delegations",
              subcommands: [
                {
                  name: "claim",
                  description: "Claim rewards accumulated during the delegation of unlocked tokens",
                  options: [
                    {
                      name: "--identity-key",
                      args: {
                        name: "identity-key",
                      },
                    },
                    {
                      name: "--mnemonic",
                      description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
                      args: {
                        name: "mnemonic",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--config-env-file",
                      description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
                      args: {
                        name: "config-env-file",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--nyxd-url",
                      description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
                      args: {
                        name: "nyxd-url",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--validator-api-url",
                      description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
                      args: {
                        name: "validator-api-url",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--mixnet-contract-address",
                      description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
                      args: {
                        name: "mixnet-contract-address",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--vesting-contract-address",
                      description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
                      args: {
                        name: "vesting-contract-address",
                        isOptional: true,
                      },
                    },
                    {
                      name: ["-h", "--help"],
                      description: "Print help information",
                    },
                  ],
                },
                {
                  name: "vesting-claim",
                  description: "Claim rewards accumulated during the delegation of locked tokens",
                  options: [
                    {
                      name: "--identity",
                      args: {
                        name: "identity",
                      },
                    },
                    {
                      name: "--mnemonic",
                      description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
                      args: {
                        name: "mnemonic",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--config-env-file",
                      description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
                      args: {
                        name: "config-env-file",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--nyxd-url",
                      description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
                      args: {
                        name: "nyxd-url",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--validator-api-url",
                      description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
                      args: {
                        name: "validator-api-url",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--mixnet-contract-address",
                      description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
                      args: {
                        name: "mixnet-contract-address",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--vesting-contract-address",
                      description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
                      args: {
                        name: "vesting-contract-address",
                        isOptional: true,
                      },
                    },
                    {
                      name: ["-h", "--help"],
                      description: "Print help information",
                    },
                  ],
                },
                {
                  name: "help",
                  description: "Print this message or the help of the given subcommand(s)",
                  options: [
                    {
                      name: "--mnemonic",
                      description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
                      args: {
                        name: "mnemonic",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--config-env-file",
                      description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
                      args: {
                        name: "config-env-file",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--nyxd-url",
                      description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
                      args: {
                        name: "nyxd-url",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--validator-api-url",
                      description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
                      args: {
                        name: "validator-api-url",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--mixnet-contract-address",
                      description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
                      args: {
                        name: "mixnet-contract-address",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--vesting-contract-address",
                      description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
                      args: {
                        name: "vesting-contract-address",
                        isOptional: true,
                      },
                    },
                  ],
                  args: {
                    name: "subcommand",
                    isOptional: true,
                  },
                },
              ],
              options: [
                {
                  name: "--mnemonic",
                  description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
                  args: {
                    name: "mnemonic",
                    isOptional: true,
                  },
                },
                {
                  name: "--config-env-file",
                  description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
                  args: {
                    name: "config-env-file",
                    isOptional: true,
                  },
                },
                {
                  name: "--nyxd-url",
                  description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
                  args: {
                    name: "nyxd-url",
                    isOptional: true,
                  },
                },
                {
                  name: "--validator-api-url",
                  description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
                  args: {
                    name: "validator-api-url",
                    isOptional: true,
                  },
                },
                {
                  name: "--mixnet-contract-address",
                  description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
                  args: {
                    name: "mixnet-contract-address",
                    isOptional: true,
                  },
                },
                {
                  name: "--vesting-contract-address",
                  description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
                  args: {
                    name: "vesting-contract-address",
                    isOptional: true,
                  },
                },
                {
                  name: ["-h", "--help"],
                  description: "Print help information",
                },
              ],
            },
            {
              name: "delegate",
              description: "Delegate to a mixnode",
              options: [
                {
                  name: "--identity-key",
                  args: {
                    name: "identity-key",
                  },
                },
                {
                  name: "--amount",
                  args: {
                    name: "amount",
                  },
                },
                {
                  name: "--mnemonic",
                  description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
                  args: {
                    name: "mnemonic",
                    isOptional: true,
                  },
                },
                {
                  name: "--config-env-file",
                  description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
                  args: {
                    name: "config-env-file",
                    isOptional: true,
                  },
                },
                {
                  name: "--nyxd-url",
                  description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
                  args: {
                    name: "nyxd-url",
                    isOptional: true,
                  },
                },
                {
                  name: "--validator-api-url",
                  description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
                  args: {
                    name: "validator-api-url",
                    isOptional: true,
                  },
                },
                {
                  name: "--mixnet-contract-address",
                  description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
                  args: {
                    name: "mixnet-contract-address",
                    isOptional: true,
                  },
                },
                {
                  name: "--vesting-contract-address",
                  description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
                  args: {
                    name: "vesting-contract-address",
                    isOptional: true,
                  },
                },
                {
                  name: ["-h", "--help"],
                  description: "Print help information",
                },
              ],
            },
            {
              name: "undelegate",
              description: "Undelegate from a mixnode",
              options: [
                {
                  name: "--identity-key",
                  args: {
                    name: "identity-key",
                  },
                },
                {
                  name: "--mnemonic",
                  description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
                  args: {
                    name: "mnemonic",
                    isOptional: true,
                  },
                },
                {
                  name: "--config-env-file",
                  description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
                  args: {
                    name: "config-env-file",
                    isOptional: true,
                  },
                },
                {
                  name: "--nyxd-url",
                  description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
                  args: {
                    name: "nyxd-url",
                    isOptional: true,
                  },
                },
                {
                  name: "--validator-api-url",
                  description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
                  args: {
                    name: "validator-api-url",
                    isOptional: true,
                  },
                },
                {
                  name: "--mixnet-contract-address",
                  description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
                  args: {
                    name: "mixnet-contract-address",
                    isOptional: true,
                  },
                },
                {
                  name: "--vesting-contract-address",
                  description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
                  args: {
                    name: "vesting-contract-address",
                    isOptional: true,
                  },
                },
                {
                  name: ["-h", "--help"],
                  description: "Print help information",
                },
              ],
            },
            {
              name: "delegate-vesting",
              description: "Delegate to a mixnode with locked tokens",
              options: [
                {
                  name: "--identity-key",
                  args: {
                    name: "identity-key",
                  },
                },
                {
                  name: "--amount",
                  args: {
                    name: "amount",
                  },
                },
                {
                  name: "--mnemonic",
                  description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
                  args: {
                    name: "mnemonic",
                    isOptional: true,
                  },
                },
                {
                  name: "--config-env-file",
                  description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
                  args: {
                    name: "config-env-file",
                    isOptional: true,
                  },
                },
                {
                  name: "--nyxd-url",
                  description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
                  args: {
                    name: "nyxd-url",
                    isOptional: true,
                  },
                },
                {
                  name: "--validator-api-url",
                  description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
                  args: {
                    name: "validator-api-url",
                    isOptional: true,
                  },
                },
                {
                  name: "--mixnet-contract-address",
                  description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
                  args: {
                    name: "mixnet-contract-address",
                    isOptional: true,
                  },
                },
                {
                  name: "--vesting-contract-address",
                  description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
                  args: {
                    name: "vesting-contract-address",
                    isOptional: true,
                  },
                },
                {
                  name: ["-h", "--help"],
                  description: "Print help information",
                },
              ],
            },
            {
              name: "undelegate-vesting",
              description: "Undelegate from a mixnode (when originally using locked tokens)",
              options: [
                {
                  name: "--identity-key",
                  args: {
                    name: "identity-key",
                  },
                },
                {
                  name: "--mnemonic",
                  description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
                  args: {
                    name: "mnemonic",
                    isOptional: true,
                  },
                },
                {
                  name: "--config-env-file",
                  description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
                  args: {
                    name: "config-env-file",
                    isOptional: true,
                  },
                },
                {
                  name: "--nyxd-url",
                  description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
                  args: {
                    name: "nyxd-url",
                    isOptional: true,
                  },
                },
                {
                  name: "--validator-api-url",
                  description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
                  args: {
                    name: "validator-api-url",
                    isOptional: true,
                  },
                },
                {
                  name: "--mixnet-contract-address",
                  description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
                  args: {
                    name: "mixnet-contract-address",
                    isOptional: true,
                  },
                },
                {
                  name: "--vesting-contract-address",
                  description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
                  args: {
                    name: "vesting-contract-address",
                    isOptional: true,
                  },
                },
                {
                  name: ["-h", "--help"],
                  description: "Print help information",
                },
              ],
            },
            {
              name: "help",
              description: "Print this message or the help of the given subcommand(s)",
              options: [
                {
                  name: "--mnemonic",
                  description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
                  args: {
                    name: "mnemonic",
                    isOptional: true,
                  },
                },
                {
                  name: "--config-env-file",
                  description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
                  args: {
                    name: "config-env-file",
                    isOptional: true,
                  },
                },
                {
                  name: "--nyxd-url",
                  description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
                  args: {
                    name: "nyxd-url",
                    isOptional: true,
                  },
                },
                {
                  name: "--validator-api-url",
                  description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
                  args: {
                    name: "validator-api-url",
                    isOptional: true,
                  },
                },
                {
                  name: "--mixnet-contract-address",
                  description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
                  args: {
                    name: "mixnet-contract-address",
                    isOptional: true,
                  },
                },
                {
                  name: "--vesting-contract-address",
                  description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
                  args: {
                    name: "vesting-contract-address",
                    isOptional: true,
                  },
                },
              ],
              args: {
                name: "subcommand",
                isOptional: true,
              },
            },
          ],
          options: [
            {
              name: "--mnemonic",
              description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
              args: {
                name: "mnemonic",
                isOptional: true,
              },
            },
            {
              name: "--config-env-file",
              description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
              args: {
                name: "config-env-file",
                isOptional: true,
              },
            },
            {
              name: "--nyxd-url",
              description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
              args: {
                name: "nyxd-url",
                isOptional: true,
              },
            },
            {
              name: "--validator-api-url",
              description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
              args: {
                name: "validator-api-url",
                isOptional: true,
              },
            },
            {
              name: "--mixnet-contract-address",
              description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
              args: {
                name: "mixnet-contract-address",
                isOptional: true,
              },
            },
            {
              name: "--vesting-contract-address",
              description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
              args: {
                name: "vesting-contract-address",
                isOptional: true,
              },
            },
            {
              name: ["-h", "--help"],
              description: "Print help information",
            },
          ],
        },
        {
          name: "operators",
          description: "Manage a mixnode or gateway you operate",
          subcommands: [
            {
              name: "mixnode",
              description: "Manage your mixnode",
              subcommands: [
                {
                  name: "keys",
                  description: "Operations for mixnode keys",
                  subcommands: [
                    {
                      name: "decode-mixnode-key",
                      description: "Decode a mixnode key",
                      options: [
                        {
                          name: ["-k", "--key"],
                          args: {
                            name: "key",
                          },
                        },
                        {
                          name: "--mnemonic",
                          description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
                          args: {
                            name: "mnemonic",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--config-env-file",
                          description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
                          args: {
                            name: "config-env-file",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--nyxd-url",
                          description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
                          args: {
                            name: "nyxd-url",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--validator-api-url",
                          description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
                          args: {
                            name: "validator-api-url",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--mixnet-contract-address",
                          description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
                          args: {
                            name: "mixnet-contract-address",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--vesting-contract-address",
                          description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
                          args: {
                            name: "vesting-contract-address",
                            isOptional: true,
                          },
                        },
                        {
                          name: ["-h", "--help"],
                          description: "Print help information",
                        },
                      ],
                    },
                    {
                      name: "help",
                      description: "Print this message or the help of the given subcommand(s)",
                      options: [
                        {
                          name: "--mnemonic",
                          description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
                          args: {
                            name: "mnemonic",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--config-env-file",
                          description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
                          args: {
                            name: "config-env-file",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--nyxd-url",
                          description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
                          args: {
                            name: "nyxd-url",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--validator-api-url",
                          description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
                          args: {
                            name: "validator-api-url",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--mixnet-contract-address",
                          description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
                          args: {
                            name: "mixnet-contract-address",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--vesting-contract-address",
                          description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
                          args: {
                            name: "vesting-contract-address",
                            isOptional: true,
                          },
                        },
                      ],
                      args: {
                        name: "subcommand",
                        isOptional: true,
                      },
                    },
                  ],
                  options: [
                    {
                      name: "--mnemonic",
                      description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
                      args: {
                        name: "mnemonic",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--config-env-file",
                      description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
                      args: {
                        name: "config-env-file",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--nyxd-url",
                      description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
                      args: {
                        name: "nyxd-url",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--validator-api-url",
                      description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
                      args: {
                        name: "validator-api-url",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--mixnet-contract-address",
                      description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
                      args: {
                        name: "mixnet-contract-address",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--vesting-contract-address",
                      description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
                      args: {
                        name: "vesting-contract-address",
                        isOptional: true,
                      },
                    },
                    {
                      name: ["-h", "--help"],
                      description: "Print help information",
                    },
                  ],
                },
                {
                  name: "rewards",
                  description: "Manage your mixnode operator rewards",
                  subcommands: [
                    {
                      name: "claim",
                      description: "Claim rewards",
                      options: [
                        {
                          name: "--mnemonic",
                          description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
                          args: {
                            name: "mnemonic",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--config-env-file",
                          description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
                          args: {
                            name: "config-env-file",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--nyxd-url",
                          description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
                          args: {
                            name: "nyxd-url",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--validator-api-url",
                          description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
                          args: {
                            name: "validator-api-url",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--mixnet-contract-address",
                          description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
                          args: {
                            name: "mixnet-contract-address",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--vesting-contract-address",
                          description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
                          args: {
                            name: "vesting-contract-address",
                            isOptional: true,
                          },
                        },
                        {
                          name: ["-h", "--help"],
                          description: "Print help information",
                        },
                      ],
                    },
                    {
                      name: "vesting-claim",
                      description: "Claim rewards for a mixnode bonded with locked tokens",
                      options: [
                        {
                          name: "--gas",
                          args: {
                            name: "gas",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--mnemonic",
                          description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
                          args: {
                            name: "mnemonic",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--config-env-file",
                          description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
                          args: {
                            name: "config-env-file",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--nyxd-url",
                          description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
                          args: {
                            name: "nyxd-url",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--validator-api-url",
                          description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
                          args: {
                            name: "validator-api-url",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--mixnet-contract-address",
                          description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
                          args: {
                            name: "mixnet-contract-address",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--vesting-contract-address",
                          description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
                          args: {
                            name: "vesting-contract-address",
                            isOptional: true,
                          },
                        },
                        {
                          name: ["-h", "--help"],
                          description: "Print help information",
                        },
                      ],
                    },
                    {
                      name: "help",
                      description: "Print this message or the help of the given subcommand(s)",
                      options: [
                        {
                          name: "--mnemonic",
                          description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
                          args: {
                            name: "mnemonic",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--config-env-file",
                          description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
                          args: {
                            name: "config-env-file",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--nyxd-url",
                          description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
                          args: {
                            name: "nyxd-url",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--validator-api-url",
                          description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
                          args: {
                            name: "validator-api-url",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--mixnet-contract-address",
                          description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
                          args: {
                            name: "mixnet-contract-address",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--vesting-contract-address",
                          description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
                          args: {
                            name: "vesting-contract-address",
                            isOptional: true,
                          },
                        },
                      ],
                      args: {
                        name: "subcommand",
                        isOptional: true,
                      },
                    },
                  ],
                  options: [
                    {
                      name: "--mnemonic",
                      description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
                      args: {
                        name: "mnemonic",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--config-env-file",
                      description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
                      args: {
                        name: "config-env-file",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--nyxd-url",
                      description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
                      args: {
                        name: "nyxd-url",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--validator-api-url",
                      description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
                      args: {
                        name: "validator-api-url",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--mixnet-contract-address",
                      description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
                      args: {
                        name: "mixnet-contract-address",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--vesting-contract-address",
                      description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
                      args: {
                        name: "vesting-contract-address",
                        isOptional: true,
                      },
                    },
                    {
                      name: ["-h", "--help"],
                      description: "Print help information",
                    },
                  ],
                },
                {
                  name: "settings",
                  description: "Manage your mixnode settings stored in the directory",
                  subcommands: [
                    {
                      name: "update-profit-percentage",
                      description: "Update profit percentage",
                      options: [
                        {
                          name: "--profit-percent",
                          args: {
                            name: "profit-percent",
                          },
                        },
                        {
                          name: "--mnemonic",
                          description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
                          args: {
                            name: "mnemonic",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--config-env-file",
                          description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
                          args: {
                            name: "config-env-file",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--nyxd-url",
                          description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
                          args: {
                            name: "nyxd-url",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--validator-api-url",
                          description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
                          args: {
                            name: "validator-api-url",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--mixnet-contract-address",
                          description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
                          args: {
                            name: "mixnet-contract-address",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--vesting-contract-address",
                          description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
                          args: {
                            name: "vesting-contract-address",
                            isOptional: true,
                          },
                        },
                        {
                          name: ["-h", "--help"],
                          description: "Print help information",
                        },
                      ],
                    },
                    {
                      name: "vesting-update-profit-percentage",
                      description: "Update profit percentage for a mixnode bonded with locked tokens",
                      options: [
                        {
                          name: "--profit-percent",
                          args: {
                            name: "profit-percent",
                          },
                        },
                        {
                          name: "--gas",
                          args: {
                            name: "gas",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--mnemonic",
                          description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
                          args: {
                            name: "mnemonic",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--config-env-file",
                          description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
                          args: {
                            name: "config-env-file",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--nyxd-url",
                          description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
                          args: {
                            name: "nyxd-url",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--validator-api-url",
                          description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
                          args: {
                            name: "validator-api-url",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--mixnet-contract-address",
                          description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
                          args: {
                            name: "mixnet-contract-address",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--vesting-contract-address",
                          description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
                          args: {
                            name: "vesting-contract-address",
                            isOptional: true,
                          },
                        },
                        {
                          name: ["-h", "--help"],
                          description: "Print help information",
                        },
                      ],
                    },
                    {
                      name: "help",
                      description: "Print this message or the help of the given subcommand(s)",
                      options: [
                        {
                          name: "--mnemonic",
                          description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
                          args: {
                            name: "mnemonic",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--config-env-file",
                          description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
                          args: {
                            name: "config-env-file",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--nyxd-url",
                          description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
                          args: {
                            name: "nyxd-url",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--validator-api-url",
                          description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
                          args: {
                            name: "validator-api-url",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--mixnet-contract-address",
                          description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
                          args: {
                            name: "mixnet-contract-address",
                            isOptional: true,
                          },
                        },
                        {
                          name: "--vesting-contract-address",
                          description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
                          args: {
                            name: "vesting-contract-address",
                            isOptional: true,
                          },
                        },
                      ],
                      args: {
                        name: "subcommand",
                        isOptional: true,
                      },
                    },
                  ],
                  options: [
                    {
                      name: "--mnemonic",
                      description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
                      args: {
                        name: "mnemonic",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--config-env-file",
                      description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
                      args: {
                        name: "config-env-file",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--nyxd-url",
                      description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
                      args: {
                        name: "nyxd-url",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--validator-api-url",
                      description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
                      args: {
                        name: "validator-api-url",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--mixnet-contract-address",
                      description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
                      args: {
                        name: "mixnet-contract-address",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--vesting-contract-address",
                      description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
                      args: {
                        name: "vesting-contract-address",
                        isOptional: true,
                      },
                    },
                    {
                      name: ["-h", "--help"],
                      description: "Print help information",
                    },
                  ],
                },
                {
                  name: "bond",
                  description: "Bond to a mixnode",
                  options: [
                    {
                      name: "--host",
                      args: {
                        name: "host",
                      },
                    },
                    {
                      name: "--signature",
                      args: {
                        name: "signature",
                      },
                    },
                    {
                      name: "--mix-port",
                      args: {
                        name: "mix-port",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--verloc-port",
                      args: {
                        name: "verloc-port",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--http-api-port",
                      args: {
                        name: "http-api-port",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--sphinx-key",
                      args: {
                        name: "sphinx-key",
                      },
                    },
                    {
                      name: "--identity-key",
                      args: {
                        name: "identity-key",
                      },
                    },
                    {
                      name: "--version",
                      args: {
                        name: "version",
                      },
                    },
                    {
                      name: "--profit-margin-percent",
                      args: {
                        name: "profit-margin-percent",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--amount",
                      description: "bonding amount in current DENOMINATION (so it would be 'unym', rather than 'nym')",
                      args: {
                        name: "amount",
                      },
                    },
                    {
                      name: "--mnemonic",
                      description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
                      args: {
                        name: "mnemonic",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--config-env-file",
                      description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
                      args: {
                        name: "config-env-file",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--nyxd-url",
                      description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
                      args: {
                        name: "nyxd-url",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--validator-api-url",
                      description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
                      args: {
                        name: "validator-api-url",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--mixnet-contract-address",
                      description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
                      args: {
                        name: "mixnet-contract-address",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--vesting-contract-address",
                      description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
                      args: {
                        name: "vesting-contract-address",
                        isOptional: true,
                      },
                    },
                    {
                      name: ["-f", "--force"],
                    },
                    {
                      name: ["-h", "--help"],
                      description: "Print help information",
                    },
                  ],
                },
                {
                  name: "unbound",
                  description: "Unbound from a mixnode",
                  options: [
                    {
                      name: "--mnemonic",
                      description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
                      args: {
                        name: "mnemonic",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--config-env-file",
                      description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
                      args: {
                        name: "config-env-file",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--nyxd-url",
                      description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
                      args: {
                        name: "nyxd-url",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--validator-api-url",
                      description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
                      args: {
                        name: "validator-api-url",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--mixnet-contract-address",
                      description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
                      args: {
                        name: "mixnet-contract-address",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--vesting-contract-address",
                      description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
                      args: {
                        name: "vesting-contract-address",
                        isOptional: true,
                      },
                    },
                    {
                      name: ["-h", "--help"],
                      description: "Print help information",
                    },
                  ],
                },
                {
                  name: "bond-vesting",
                  description: "Bond to a mixnode with locked tokens",
                  options: [
                    {
                      name: "--host",
                      args: {
                        name: "host",
                      },
                    },
                    {
                      name: "--signature",
                      args: {
                        name: "signature",
                      },
                    },
                    {
                      name: "--mix-port",
                      args: {
                        name: "mix-port",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--verloc-port",
                      args: {
                        name: "verloc-port",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--http-api-port",
                      args: {
                        name: "http-api-port",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--sphinx-key",
                      args: {
                        name: "sphinx-key",
                      },
                    },
                    {
                      name: "--identity-key",
                      args: {
                        name: "identity-key",
                      },
                    },
                    {
                      name: "--version",
                      args: {
                        name: "version",
                      },
                    },
                    {
                      name: "--profit-margin-percent",
                      args: {
                        name: "profit-margin-percent",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--amount",
                      description: "bonding amount in current DENOMINATION (so it would be 'unym', rather than 'nym')",
                      args: {
                        name: "amount",
                      },
                    },
                    {
                      name: "--gas",
                      args: {
                        name: "gas",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--mnemonic",
                      description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
                      args: {
                        name: "mnemonic",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--config-env-file",
                      description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
                      args: {
                        name: "config-env-file",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--nyxd-url",
                      description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
                      args: {
                        name: "nyxd-url",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--validator-api-url",
                      description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
                      args: {
                        name: "validator-api-url",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--mixnet-contract-address",
                      description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
                      args: {
                        name: "mixnet-contract-address",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--vesting-contract-address",
                      description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
                      args: {
                        name: "vesting-contract-address",
                        isOptional: true,
                      },
                    },
                    {
                      name: ["-f", "--force"],
                    },
                    {
                      name: ["-h", "--help"],
                      description: "Print help information",
                    },
                  ],
                },
                {
                  name: "unbound-vesting",
                  description: "Unbound from a mixnode (when originally using locked tokens)",
                  options: [
                    {
                      name: "--gas",
                      args: {
                        name: "gas",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--mnemonic",
                      description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
                      args: {
                        name: "mnemonic",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--config-env-file",
                      description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
                      args: {
                        name: "config-env-file",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--nyxd-url",
                      description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
                      args: {
                        name: "nyxd-url",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--validator-api-url",
                      description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
                      args: {
                        name: "validator-api-url",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--mixnet-contract-address",
                      description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
                      args: {
                        name: "mixnet-contract-address",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--vesting-contract-address",
                      description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
                      args: {
                        name: "vesting-contract-address",
                        isOptional: true,
                      },
                    },
                    {
                      name: ["-h", "--help"],
                      description: "Print help information",
                    },
                  ],
                },
                {
                  name: "help",
                  description: "Print this message or the help of the given subcommand(s)",
                  options: [
                    {
                      name: "--mnemonic",
                      description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
                      args: {
                        name: "mnemonic",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--config-env-file",
                      description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
                      args: {
                        name: "config-env-file",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--nyxd-url",
                      description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
                      args: {
                        name: "nyxd-url",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--validator-api-url",
                      description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
                      args: {
                        name: "validator-api-url",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--mixnet-contract-address",
                      description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
                      args: {
                        name: "mixnet-contract-address",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--vesting-contract-address",
                      description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
                      args: {
                        name: "vesting-contract-address",
                        isOptional: true,
                      },
                    },
                  ],
                  args: {
                    name: "subcommand",
                    isOptional: true,
                  },
                },
              ],
              options: [
                {
                  name: "--mnemonic",
                  description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
                  args: {
                    name: "mnemonic",
                    isOptional: true,
                  },
                },
                {
                  name: "--config-env-file",
                  description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
                  args: {
                    name: "config-env-file",
                    isOptional: true,
                  },
                },
                {
                  name: "--nyxd-url",
                  description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
                  args: {
                    name: "nyxd-url",
                    isOptional: true,
                  },
                },
                {
                  name: "--validator-api-url",
                  description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
                  args: {
                    name: "validator-api-url",
                    isOptional: true,
                  },
                },
                {
                  name: "--mixnet-contract-address",
                  description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
                  args: {
                    name: "mixnet-contract-address",
                    isOptional: true,
                  },
                },
                {
                  name: "--vesting-contract-address",
                  description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
                  args: {
                    name: "vesting-contract-address",
                    isOptional: true,
                  },
                },
                {
                  name: ["-h", "--help"],
                  description: "Print help information",
                },
              ],
            },
            {
              name: "gateway",
              description: "Manage your gateway",
              subcommands: [
                {
                  name: "bond",
                  description: "Bond to a gateway",
                  options: [
                    {
                      name: "--host",
                      args: {
                        name: "host",
                      },
                    },
                    {
                      name: "--signature",
                      args: {
                        name: "signature",
                      },
                    },
                    {
                      name: "--mix-port",
                      args: {
                        name: "mix-port",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--clients-port",
                      args: {
                        name: "clients-port",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--location",
                      args: {
                        name: "location",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--sphinx-key",
                      args: {
                        name: "sphinx-key",
                      },
                    },
                    {
                      name: "--identity-key",
                      args: {
                        name: "identity-key",
                      },
                    },
                    {
                      name: "--version",
                      args: {
                        name: "version",
                      },
                    },
                    {
                      name: "--amount",
                      description: "bonding amount in current DENOMINATION (so it would be 'unym', rather than 'nym')",
                      args: {
                        name: "amount",
                      },
                    },
                    {
                      name: "--mnemonic",
                      description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
                      args: {
                        name: "mnemonic",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--config-env-file",
                      description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
                      args: {
                        name: "config-env-file",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--nyxd-url",
                      description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
                      args: {
                        name: "nyxd-url",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--validator-api-url",
                      description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
                      args: {
                        name: "validator-api-url",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--mixnet-contract-address",
                      description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
                      args: {
                        name: "mixnet-contract-address",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--vesting-contract-address",
                      description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
                      args: {
                        name: "vesting-contract-address",
                        isOptional: true,
                      },
                    },
                    {
                      name: ["-f", "--force"],
                    },
                    {
                      name: ["-h", "--help"],
                      description: "Print help information",
                    },
                  ],
                },
                {
                  name: "unbound",
                  description: "Unbound from a gateway",
                  options: [
                    {
                      name: "--mnemonic",
                      description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
                      args: {
                        name: "mnemonic",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--config-env-file",
                      description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
                      args: {
                        name: "config-env-file",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--nyxd-url",
                      description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
                      args: {
                        name: "nyxd-url",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--validator-api-url",
                      description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
                      args: {
                        name: "validator-api-url",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--mixnet-contract-address",
                      description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
                      args: {
                        name: "mixnet-contract-address",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--vesting-contract-address",
                      description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
                      args: {
                        name: "vesting-contract-address",
                        isOptional: true,
                      },
                    },
                    {
                      name: ["-h", "--help"],
                      description: "Print help information",
                    },
                  ],
                },
                {
                  name: "vesting-bond",
                  description: "Bond to a gateway with locked tokens",
                  options: [
                    {
                      name: "--host",
                      args: {
                        name: "host",
                      },
                    },
                    {
                      name: "--signature",
                      args: {
                        name: "signature",
                      },
                    },
                    {
                      name: "--mix-port",
                      args: {
                        name: "mix-port",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--clients-port",
                      args: {
                        name: "clients-port",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--location",
                      args: {
                        name: "location",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--sphinx-key",
                      args: {
                        name: "sphinx-key",
                      },
                    },
                    {
                      name: "--identity-key",
                      args: {
                        name: "identity-key",
                      },
                    },
                    {
                      name: "--version",
                      args: {
                        name: "version",
                      },
                    },
                    {
                      name: "--amount",
                      description: "bonding amount in current DENOMINATION (so it would be 'unym', rather than 'nym')",
                      args: {
                        name: "amount",
                      },
                    },
                    {
                      name: "--mnemonic",
                      description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
                      args: {
                        name: "mnemonic",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--config-env-file",
                      description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
                      args: {
                        name: "config-env-file",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--nyxd-url",
                      description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
                      args: {
                        name: "nyxd-url",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--validator-api-url",
                      description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
                      args: {
                        name: "validator-api-url",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--mixnet-contract-address",
                      description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
                      args: {
                        name: "mixnet-contract-address",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--vesting-contract-address",
                      description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
                      args: {
                        name: "vesting-contract-address",
                        isOptional: true,
                      },
                    },
                    {
                      name: ["-f", "--force"],
                    },
                    {
                      name: ["-h", "--help"],
                      description: "Print help information",
                    },
                  ],
                },
                {
                  name: "vesting-unbound",
                  description: "Unbound from a gateway (when originally using locked tokens)",
                  options: [
                    {
                      name: "--mnemonic",
                      description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
                      args: {
                        name: "mnemonic",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--config-env-file",
                      description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
                      args: {
                        name: "config-env-file",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--nyxd-url",
                      description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
                      args: {
                        name: "nyxd-url",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--validator-api-url",
                      description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
                      args: {
                        name: "validator-api-url",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--mixnet-contract-address",
                      description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
                      args: {
                        name: "mixnet-contract-address",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--vesting-contract-address",
                      description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
                      args: {
                        name: "vesting-contract-address",
                        isOptional: true,
                      },
                    },
                    {
                      name: ["-h", "--help"],
                      description: "Print help information",
                    },
                  ],
                },
                {
                  name: "help",
                  description: "Print this message or the help of the given subcommand(s)",
                  options: [
                    {
                      name: "--mnemonic",
                      description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
                      args: {
                        name: "mnemonic",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--config-env-file",
                      description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
                      args: {
                        name: "config-env-file",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--nyxd-url",
                      description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
                      args: {
                        name: "nyxd-url",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--validator-api-url",
                      description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
                      args: {
                        name: "validator-api-url",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--mixnet-contract-address",
                      description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
                      args: {
                        name: "mixnet-contract-address",
                        isOptional: true,
                      },
                    },
                    {
                      name: "--vesting-contract-address",
                      description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
                      args: {
                        name: "vesting-contract-address",
                        isOptional: true,
                      },
                    },
                  ],
                  args: {
                    name: "subcommand",
                    isOptional: true,
                  },
                },
              ],
              options: [
                {
                  name: "--mnemonic",
                  description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
                  args: {
                    name: "mnemonic",
                    isOptional: true,
                  },
                },
                {
                  name: "--config-env-file",
                  description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
                  args: {
                    name: "config-env-file",
                    isOptional: true,
                  },
                },
                {
                  name: "--nyxd-url",
                  description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
                  args: {
                    name: "nyxd-url",
                    isOptional: true,
                  },
                },
                {
                  name: "--validator-api-url",
                  description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
                  args: {
                    name: "validator-api-url",
                    isOptional: true,
                  },
                },
                {
                  name: "--mixnet-contract-address",
                  description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
                  args: {
                    name: "mixnet-contract-address",
                    isOptional: true,
                  },
                },
                {
                  name: "--vesting-contract-address",
                  description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
                  args: {
                    name: "vesting-contract-address",
                    isOptional: true,
                  },
                },
                {
                  name: ["-h", "--help"],
                  description: "Print help information",
                },
              ],
            },
            {
              name: "help",
              description: "Print this message or the help of the given subcommand(s)",
              options: [
                {
                  name: "--mnemonic",
                  description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
                  args: {
                    name: "mnemonic",
                    isOptional: true,
                  },
                },
                {
                  name: "--config-env-file",
                  description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
                  args: {
                    name: "config-env-file",
                    isOptional: true,
                  },
                },
                {
                  name: "--nyxd-url",
                  description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
                  args: {
                    name: "nyxd-url",
                    isOptional: true,
                  },
                },
                {
                  name: "--validator-api-url",
                  description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
                  args: {
                    name: "validator-api-url",
                    isOptional: true,
                  },
                },
                {
                  name: "--mixnet-contract-address",
                  description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
                  args: {
                    name: "mixnet-contract-address",
                    isOptional: true,
                  },
                },
                {
                  name: "--vesting-contract-address",
                  description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
                  args: {
                    name: "vesting-contract-address",
                    isOptional: true,
                  },
                },
              ],
              args: {
                name: "subcommand",
                isOptional: true,
              },
            },
          ],
          options: [
            {
              name: "--mnemonic",
              description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
              args: {
                name: "mnemonic",
                isOptional: true,
              },
            },
            {
              name: "--config-env-file",
              description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
              args: {
                name: "config-env-file",
                isOptional: true,
              },
            },
            {
              name: "--nyxd-url",
              description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
              args: {
                name: "nyxd-url",
                isOptional: true,
              },
            },
            {
              name: "--validator-api-url",
              description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
              args: {
                name: "validator-api-url",
                isOptional: true,
              },
            },
            {
              name: "--mixnet-contract-address",
              description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
              args: {
                name: "mixnet-contract-address",
                isOptional: true,
              },
            },
            {
              name: "--vesting-contract-address",
              description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
              args: {
                name: "vesting-contract-address",
                isOptional: true,
              },
            },
            {
              name: ["-h", "--help"],
              description: "Print help information",
            },
          ],
        },
        {
          name: "help",
          description: "Print this message or the help of the given subcommand(s)",
          options: [
            {
              name: "--mnemonic",
              description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
              args: {
                name: "mnemonic",
                isOptional: true,
              },
            },
            {
              name: "--config-env-file",
              description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
              args: {
                name: "config-env-file",
                isOptional: true,
              },
            },
            {
              name: "--nyxd-url",
              description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
              args: {
                name: "nyxd-url",
                isOptional: true,
              },
            },
            {
              name: "--validator-api-url",
              description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
              args: {
                name: "validator-api-url",
                isOptional: true,
              },
            },
            {
              name: "--mixnet-contract-address",
              description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
              args: {
                name: "mixnet-contract-address",
                isOptional: true,
              },
            },
            {
              name: "--vesting-contract-address",
              description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
              args: {
                name: "vesting-contract-address",
                isOptional: true,
              },
            },
          ],
          args: {
            name: "subcommand",
            isOptional: true,
          },
        },
      ],
      options: [
        {
          name: "--mnemonic",
          description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
          args: {
            name: "mnemonic",
            isOptional: true,
          },
        },
        {
          name: "--config-env-file",
          description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
          args: {
            name: "config-env-file",
            isOptional: true,
          },
        },
        {
          name: "--nyxd-url",
          description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
          args: {
            name: "nyxd-url",
            isOptional: true,
          },
        },
        {
          name: "--validator-api-url",
          description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
          args: {
            name: "validator-api-url",
            isOptional: true,
          },
        },
        {
          name: "--mixnet-contract-address",
          description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
          args: {
            name: "mixnet-contract-address",
            isOptional: true,
          },
        },
        {
          name: "--vesting-contract-address",
          description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
          args: {
            name: "vesting-contract-address",
            isOptional: true,
          },
        },
        {
          name: ["-h", "--help"],
          description: "Print help information",
        },
      ],
    },
    {
      name: "generate-fig",
      description: "Generates shell completion",
      options: [
        {
          name: "--mnemonic",
          description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
          args: {
            name: "mnemonic",
            isOptional: true,
          },
        },
        {
          name: "--config-env-file",
          description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
          args: {
            name: "config-env-file",
            isOptional: true,
          },
        },
        {
          name: "--nyxd-url",
          description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
          args: {
            name: "nyxd-url",
            isOptional: true,
          },
        },
        {
          name: "--validator-api-url",
          description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
          args: {
            name: "validator-api-url",
            isOptional: true,
          },
        },
        {
          name: "--mixnet-contract-address",
          description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
          args: {
            name: "mixnet-contract-address",
            isOptional: true,
          },
        },
        {
          name: "--vesting-contract-address",
          description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
          args: {
            name: "vesting-contract-address",
            isOptional: true,
          },
        },
        {
          name: ["-h", "--help"],
          description: "Print help information",
        },
      ],
    },
    {
      name: "help",
      description: "Print this message or the help of the given subcommand(s)",
      options: [
        {
          name: "--mnemonic",
          description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
          args: {
            name: "mnemonic",
            isOptional: true,
          },
        },
        {
          name: "--config-env-file",
          description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
          args: {
            name: "config-env-file",
            isOptional: true,
          },
        },
        {
          name: "--nyxd-url",
          description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
          args: {
            name: "nyxd-url",
            isOptional: true,
          },
        },
        {
          name: "--validator-api-url",
          description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
          args: {
            name: "validator-api-url",
            isOptional: true,
          },
        },
        {
          name: "--mixnet-contract-address",
          description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
          args: {
            name: "mixnet-contract-address",
            isOptional: true,
          },
        },
        {
          name: "--vesting-contract-address",
          description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
          args: {
            name: "vesting-contract-address",
            isOptional: true,
          },
        },
      ],
      args: {
        name: "subcommand",
        isOptional: true,
      },
    },
  ],
  options: [
    {
      name: "--mnemonic",
      description: "Provide the mnemonic for your account. You can also provide this is an env var called MNEMONIC.",
      args: {
        name: "mnemonic",
        isOptional: true,
      },
    },
    {
      name: "--config-env-file",
      description: "Overrides configuration as a file of environment variables. Note: individual env vars take precedence over this file.",
      args: {
        name: "config-env-file",
        isOptional: true,
      },
    },
    {
      name: "--nyxd-url",
      description: "Overrides the nyxd URL provided either as an environment variable nyxd_VALIDATOR or in a config file",
      args: {
        name: "nyxd-url",
        isOptional: true,
      },
    },
    {
      name: "--validator-api-url",
      description: "Overrides the validator API URL provided either as an environment variable API_VALIDATOR or in a config file",
      args: {
        name: "validator-api-url",
        isOptional: true,
      },
    },
    {
      name: "--mixnet-contract-address",
      description: "Overrides the mixnet contract address provided either as an environment variable or in a config file",
      args: {
        name: "mixnet-contract-address",
        isOptional: true,
      },
    },
    {
      name: "--vesting-contract-address",
      description: "Overrides the vesting contract address provided either as an environment variable or in a config file",
      args: {
        name: "vesting-contract-address",
        isOptional: true,
      },
    },
    {
      name: ["-h", "--help"],
      description: "Print help information",
    },
  ],
};

export default completion;
