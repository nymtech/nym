```sh
const completion: Fig.Spec = {
  name: "nym-socks5-client",
  description: "A SOCKS5 localhost proxy that converts incoming messages to Sphinx and sends them to a Nym address",
  subcommands: [
    {
      name: "init",
      description: "Initialise a Nym client. Do this first!",
      options: [
        {
          name: "--id",
          description: "Id of client we want to create config for",
          isRepeatable: true,
          args: {
            name: "id",
          },
        },
        {
          name: "--gateway",
          description: "Id of the gateway we are going to connect to",
          isRepeatable: true,
          args: {
            name: "gateway",
            isOptional: true,
          },
        },
        {
          name: "--nyxd-urls",
          description: "Comma separated list of rest endpoints of the nyxd validators",
          hidden: true,
          isRepeatable: true,
          args: {
            name: "nyxd_urls",
            isOptional: true,
          },
        },
        {
          name: "--nym-apis",
          description: "Comma separated list of rest endpoints of the API validators",
          isRepeatable: true,
          args: {
            name: "nym_apis",
            isOptional: true,
          },
        },
        {
          name: "--custom-mixnet",
          description: "Path to .json file containing custom network specification",
          hidden: true,
          isRepeatable: true,
          args: {
            name: "custom_mixnet",
            isOptional: true,
            template: "filepaths",
          },
        },
        {
          name: "--enabled-credentials-mode",
          description: "Set this client to work in a enabled credentials mode that would attempt to use gateway with bandwidth credential requirement",
          hidden: true,
          isRepeatable: true,
          args: {
            name: "enabled_credentials_mode",
            isOptional: true,
            suggestions: [
              "true",
              "false",
            ],
          },
        },
        {
          name: "--provider",
          description: "Address of the socks5 provider to send messages to",
          isRepeatable: true,
          args: {
            name: "provider",
          },
        },
        {
          name: "--use-reply-surbs",
          description: "Specifies whether this client is going to use an anonymous sender tag for communication with the service provider. While this is going to hide its actual address information, it will make the actual communication slower and consume nearly double the bandwidth as it will require sending reply SURBs",
          isRepeatable: true,
          args: {
            name: "use_reply_surbs",
            isOptional: true,
            suggestions: [
              "true",
              "false",
            ],
          },
        },
        {
          name: ["-p", "--port"],
          description: "Port for the socket to listen on in all subsequent runs",
          isRepeatable: true,
          args: {
            name: "port",
            isOptional: true,
          },
        },
        {
          name: "--host",
          description: "The custom host on which the socks5 client will be listening for requests",
          isRepeatable: true,
          args: {
            name: "host",
            isOptional: true,
          },
        },
        {
          name: ["-o", "--output"],
          isRepeatable: true,
          args: {
            name: "output",
            isOptional: true,
            suggestions: [
              "text",
              "json",
            ],
          },
        },
        {
          name: "--force-tls-gateway",
          description: "Specifies whether the client will attempt to enforce tls connection to the desired gateway",
        },
        {
          name: "--latency-based-selection",
          description: "Specifies whether the new gateway should be determined based by latency as opposed to being chosen uniformly",
          exclusiveOn: [
            "--gateway",
          ],
        },
        {
          name: "--fastmode",
          description: "Mostly debug-related option to increase default traffic rate so that you would not need to modify config post init",
        },
        {
          name: "--no-cover",
          description: "Disable loop cover traffic and the Poisson rate limiter (for debugging only)",
        },
        {
          name: ["-h", "--help"],
          description: "Print help (see more with '--help')",
        },
      ],
    },
    {
      name: "run",
      description: "Run the Nym client with provided configuration client optionally overriding set parameters",
      options: [
        {
          name: "--id",
          description: "Id of client we want to create config for",
          isRepeatable: true,
          args: {
            name: "id",
          },
        },
        {
          name: "--gateway",
          description: "Id of the gateway we want to connect to. If overridden, it is user's responsibility to ensure prior registration happened",
          isRepeatable: true,
          args: {
            name: "gateway",
            isOptional: true,
          },
        },
        {
          name: "--nyxd-urls",
          description: "Comma separated list of rest endpoints of the nyxd validators",
          hidden: true,
          isRepeatable: true,
          args: {
            name: "nyxd_urls",
            isOptional: true,
          },
        },
        {
          name: "--nym-apis",
          description: "Comma separated list of rest endpoints of the API validators",
          isRepeatable: true,
          args: {
            name: "nym_apis",
            isOptional: true,
          },
        },
        {
          name: "--custom-mixnet",
          description: "Path to .json file containing custom network specification",
          hidden: true,
          isRepeatable: true,
          args: {
            name: "custom_mixnet",
            isOptional: true,
            template: "filepaths",
          },
        },
        {
          name: "--enabled-credentials-mode",
          description: "Set this client to work in a enabled credentials mode that would attempt to use gateway with bandwidth credential requirement",
          hidden: true,
          isRepeatable: true,
          args: {
            name: "enabled_credentials_mode",
            isOptional: true,
            suggestions: [
              "true",
              "false",
            ],
          },
        },
        {
          name: "--use-anonymous-replies",
          description: "Specifies whether this client is going to use an anonymous sender tag for communication with the service provider. While this is going to hide its actual address information, it will make the actual communication slower and consume nearly double the bandwidth as it will require sending reply SURBs",
          isRepeatable: true,
          args: {
            name: "use_anonymous_replies",
            isOptional: true,
            suggestions: [
              "true",
              "false",
            ],
          },
        },
        {
          name: "--provider",
          description: "Address of the socks5 provider to send messages to",
          isRepeatable: true,
          args: {
            name: "provider",
            isOptional: true,
          },
        },
        {
          name: ["-p", "--port"],
          description: "Port for the socket to listen on",
          isRepeatable: true,
          args: {
            name: "port",
            isOptional: true,
          },
        },
        {
          name: "--host",
          description: "The custom host on which the socks5 client will be listening for requests",
          isRepeatable: true,
          args: {
            name: "host",
            isOptional: true,
          },
        },
        {
          name: "--geo-routing",
          description: "Set geo-aware mixnode selection when sending mixnet traffic, for experiments only",
          hidden: true,
          isRepeatable: true,
          args: {
            name: "geo_routing",
            isOptional: true,
          },
        },
        {
          name: "--fastmode",
          description: "Mostly debug-related option to increase default traffic rate so that you would not need to modify config post init",
        },
        {
          name: "--no-cover",
          description: "Disable loop cover traffic and the Poisson rate limiter (for debugging only)",
        },
        {
          name: "--medium-toggle",
          description: "Enable medium mixnet traffic, for experiments only. This includes things like disabling cover traffic, no per hop delays, etc",
        },
        {
          name: "--outfox",
        },
        {
          name: ["-h", "--help"],
          description: "Print help (see more with '--help')",
        },
      ],
    },
    {
      name: "ecash",
      description: "Ecash-related functionalities",
      subcommands: [
        {
          name: "show-ticket-books",
          description: "Display information associated with the imported ticketbooks,",
          options: [
            {
              name: "--id",
              description: "Id of client that is going to display the ticketbook information",
              isRepeatable: true,
              args: {
                name: "id",
              },
            },
            {
              name: ["-o", "--output"],
              isRepeatable: true,
              args: {
                name: "output",
                isOptional: true,
                suggestions: [
                  "text",
                  "json",
                ],
              },
            },
            {
              name: ["-h", "--help"],
              description: "Print help",
            },
          ],
        },
        {
          name: "import-ticket-book",
          description: "Import a pre-generated ticketbook",
          options: [
            {
              name: "--id",
              description: "Id of client that is going to import the credential",
              isRepeatable: true,
              args: {
                name: "id",
              },
            },
            {
              name: "--credential-data",
              description: "Explicitly provide the encoded credential data (as base58)",
              isRepeatable: true,
              args: {
                name: "credential_data",
                isOptional: true,
              },
            },
            {
              name: "--credential-path",
              description: "Specifies the path to file containing binary credential data",
              isRepeatable: true,
              args: {
                name: "credential_path",
                isOptional: true,
                template: "filepaths",
              },
            },
            {
              name: "--version",
              hidden: true,
              isRepeatable: true,
              args: {
                name: "version",
                isOptional: true,
              },
            },
            {
              name: "--standalone",
              description: "Specifies whether we're attempting to import a standalone ticketbook (i.e. serialised `IssuedTicketBook`)",
            },
            {
              name: "--full",
              description: "Specifies whether we're attempting to import full ticketboot (i.e. one that **might** contain required global signatures; that is serialised `ImportableTicketBook`)",
            },
            {
              name: ["-h", "--help"],
              description: "Print help",
            },
          ],
        },
        {
          name: "import-coin-index-signatures",
          description: "Import coin index signatures needed for ticketbooks",
          options: [
            {
              name: "--id",
              description: "Id of client that is going to import the signatures",
              isRepeatable: true,
              args: {
                name: "id",
              },
            },
            {
              name: "--client-config",
              description: "Config file of the client that is supposed to use the signatures",
              isRepeatable: true,
              args: {
                name: "client_config",
                template: "filepaths",
              },
            },
            {
              name: "--signatures-data",
              description: "Explicitly provide the encoded signatures data (as base58)",
              isRepeatable: true,
              args: {
                name: "signatures_data",
                isOptional: true,
              },
            },
            {
              name: "--signatures-path",
              description: "Specifies the path to file containing binary signatures data",
              isRepeatable: true,
              args: {
                name: "signatures_path",
                isOptional: true,
                template: "filepaths",
              },
            },
            {
              name: "--version",
              hidden: true,
              isRepeatable: true,
              args: {
                name: "version",
                isOptional: true,
              },
            },
            {
              name: ["-h", "--help"],
              description: "Print help",
            },
          ],
        },
        {
          name: "import-expiration-date-signatures",
          description: "Import expiration date signatures needed for ticketbooks",
          options: [
            {
              name: "--id",
              description: "Id of client that is going to import the signatures",
              isRepeatable: true,
              args: {
                name: "id",
              },
            },
            {
              name: "--client-config",
              description: "Config file of the client that is supposed to use the signatures",
              isRepeatable: true,
              args: {
                name: "client_config",
                template: "filepaths",
              },
            },
            {
              name: "--signatures-data",
              description: "Explicitly provide the encoded signatures data (as base58)",
              isRepeatable: true,
              args: {
                name: "signatures_data",
                isOptional: true,
              },
            },
            {
              name: "--signatures-path",
              description: "Specifies the path to file containing binary signatures data",
              isRepeatable: true,
              args: {
                name: "signatures_path",
                isOptional: true,
                template: "filepaths",
              },
            },
            {
              name: "--version",
              hidden: true,
              isRepeatable: true,
              args: {
                name: "version",
                isOptional: true,
              },
            },
            {
              name: ["-h", "--help"],
              description: "Print help",
            },
          ],
        },
        {
          name: "import-master-verification-key",
          description: "Import master verification key needed for ticketbooks",
          options: [
            {
              name: "--id",
              description: "Id of client that is going to import the key",
              isRepeatable: true,
              args: {
                name: "id",
              },
            },
            {
              name: "--client-config",
              description: "Config file of the client that is supposed to use the key",
              isRepeatable: true,
              args: {
                name: "client_config",
                template: "filepaths",
              },
            },
            {
              name: "--key-data",
              description: "Explicitly provide the encoded key data (as base58)",
              isRepeatable: true,
              args: {
                name: "key_data",
                isOptional: true,
              },
            },
            {
              name: "--key-path",
              description: "Specifies the path to file containing binary key data",
              isRepeatable: true,
              args: {
                name: "key_path",
                isOptional: true,
                template: "filepaths",
              },
            },
            {
              name: "--version",
              hidden: true,
              isRepeatable: true,
              args: {
                name: "version",
                isOptional: true,
              },
            },
            {
              name: ["-h", "--help"],
              description: "Print help",
            },
          ],
        },
        {
          name: "help",
          description: "Print this message or the help of the given subcommand(s)",
          subcommands: [
            {
              name: "show-ticket-books",
              description: "Display information associated with the imported ticketbooks,",
            },
            {
              name: "import-ticket-book",
              description: "Import a pre-generated ticketbook",
            },
            {
              name: "import-coin-index-signatures",
              description: "Import coin index signatures needed for ticketbooks",
            },
            {
              name: "import-expiration-date-signatures",
              description: "Import expiration date signatures needed for ticketbooks",
            },
            {
              name: "import-master-verification-key",
              description: "Import master verification key needed for ticketbooks",
            },
            {
              name: "help",
              description: "Print this message or the help of the given subcommand(s)",
            },
          ],
        },
      ],
      options: [
        {
          name: ["-h", "--help"],
          description: "Print help",
        },
      ],
    },
    {
      name: "list-gateways",
      description: "List all registered with gateways",
      options: [
        {
          name: "--id",
          description: "Id of client we want to list gateways for",
          isRepeatable: true,
          args: {
            name: "id",
          },
        },
        {
          name: ["-o", "--output"],
          isRepeatable: true,
          args: {
            name: "output",
            isOptional: true,
            suggestions: [
              "text",
              "json",
            ],
          },
        },
        {
          name: ["-h", "--help"],
          description: "Print help",
        },
      ],
    },
    {
      name: "add-gateway",
      description: "Add new gateway to this client",
      options: [
        {
          name: "--id",
          description: "Id of client we want to add gateway for",
          isRepeatable: true,
          args: {
            name: "id",
          },
        },
        {
          name: "--gateway-id",
          description: "Explicitly specify id of the gateway to register with. If unspecified, a random gateway will be chosen instead",
          isRepeatable: true,
          args: {
            name: "gateway_id",
            isOptional: true,
          },
        },
        {
          name: "--nym-apis",
          description: "Comma separated list of rest endpoints of the API validators",
          isRepeatable: true,
          args: {
            name: "nym_apis",
            isOptional: true,
          },
        },
        {
          name: "--custom-mixnet",
          description: "Path to .json file containing custom network specification",
          hidden: true,
          isRepeatable: true,
          args: {
            name: "custom_mixnet",
            isOptional: true,
            template: "filepaths",
          },
        },
        {
          name: ["-o", "--output"],
          isRepeatable: true,
          args: {
            name: "output",
            isOptional: true,
            suggestions: [
              "text",
              "json",
            ],
          },
        },
        {
          name: "--force-tls-gateway",
          description: "Specifies whether the client will attempt to enforce tls connection to the desired gateway",
        },
        {
          name: "--latency-based-selection",
          description: "Specifies whether the new gateway should be determined based by latency as opposed to being chosen uniformly",
          exclusiveOn: [
            "--gateway-id",
          ],
        },
        {
          name: "--set-active",
          description: "Specify whether this new gateway should be set as the active one",
        },
        {
          name: ["-h", "--help"],
          description: "Print help",
        },
      ],
    },
    {
      name: "switch-gateway",
      description: "Change the currently active gateway. Note that you must have already registered with the new gateway!",
      options: [
        {
          name: "--id",
          description: "Id of client we want to list gateways for",
          isRepeatable: true,
          args: {
            name: "id",
          },
        },
        {
          name: "--gateway-id",
          description: "Id of the gateway we want to switch to",
          isRepeatable: true,
          args: {
            name: "gateway_id",
          },
        },
        {
          name: ["-h", "--help"],
          description: "Print help",
        },
      ],
    },
    {
      name: "build-info",
      description: "Show build information of this binary",
      options: [
        {
          name: ["-o", "--output"],
          isRepeatable: true,
          args: {
            name: "output",
            isOptional: true,
            suggestions: [
              "text",
              "json",
            ],
          },
        },
        {
          name: ["-h", "--help"],
          description: "Print help",
        },
      ],
    },
    {
      name: "completions",
      description: "Generate shell completions",
      options: [
        {
          name: ["-h", "--help"],
          description: "Print help",
        },
      ],
      args: {
        name: "shell",
        suggestions: [
          "bash",
          "elvish",
          "fish",
          "power-shell",
          "zsh",
        ],
      },
    },
    {
      name: "generate-fig-spec",
      description: "Generate Fig specification",
      options: [
        {
          name: ["-h", "--help"],
          description: "Print help",
        },
      ],
    },
    {
      name: "help",
      description: "Print this message or the help of the given subcommand(s)",
      subcommands: [
        {
          name: "init",
          description: "Initialise a Nym client. Do this first!",
        },
        {
          name: "run",
          description: "Run the Nym client with provided configuration client optionally overriding set parameters",
        },
        {
          name: "ecash",
          description: "Ecash-related functionalities",
          subcommands: [
            {
              name: "show-ticket-books",
              description: "Display information associated with the imported ticketbooks,",
            },
            {
              name: "import-ticket-book",
              description: "Import a pre-generated ticketbook",
            },
            {
              name: "import-coin-index-signatures",
              description: "Import coin index signatures needed for ticketbooks",
            },
            {
              name: "import-expiration-date-signatures",
              description: "Import expiration date signatures needed for ticketbooks",
            },
            {
              name: "import-master-verification-key",
              description: "Import master verification key needed for ticketbooks",
            },
          ],
        },
        {
          name: "list-gateways",
          description: "List all registered with gateways",
        },
        {
          name: "add-gateway",
          description: "Add new gateway to this client",
        },
        {
          name: "switch-gateway",
          description: "Change the currently active gateway. Note that you must have already registered with the new gateway!",
        },
        {
          name: "build-info",
          description: "Show build information of this binary",
        },
        {
          name: "completions",
          description: "Generate shell completions",
        },
        {
          name: "generate-fig-spec",
          description: "Generate Fig specification",
        },
        {
          name: "help",
          description: "Print this message or the help of the given subcommand(s)",
        },
      ],
    },
  ],
  options: [
    {
      name: ["-c", "--config-env-file"],
      description: "Path pointing to an env file that configures the client",
      isRepeatable: true,
      args: {
        name: "config_env_file",
        isOptional: true,
        template: "filepaths",
      },
    },
    {
      name: "--no-banner",
      description: "Flag used for disabling the printed banner in tty",
    },
    {
      name: ["-h", "--help"],
      description: "Print help",
    },
    {
      name: ["-V", "--version"],
      description: "Print version",
    },
  ],
};

export default completion;
```
