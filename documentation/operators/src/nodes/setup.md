# Nym Node Setup

This documentation page provides a guide on how to set up and run a [NYM NODE](nym-node.md), along with explanations of available flags, commands, and examples.

## Current version

```
<!-- cmdrun ../../../../target/release/nym-node --version | grep "Build Version" | cut -b 21-26  -->
```

```admonish info
**Migrating existing nodes to `nym-node` is simple. The steps are documented below.**
```

```admonish note
If you are a `nym-mixnode` or `nym-gateway` operator and you are not familiar wwith the binary changes called *Project Smoosh*, you can read the archived [Smoosh FAQ](../archive/smoosh-faq.md) page.
```

## Usage

> Any syntax in `<>` brackets is a user's unique variable. Exchange with a corresponding name without the `<>` brackets.

To run a new node, you can simply execute the `nym-node` command without any flags. By default, the node will set necessary configurations. If you later decide to change a setting, you can use the `-w` flag.

The most crucial aspect of running the node is specifying the `--mode`, which can be one of three: `mixnode`, `entry-gateway`, and `exit-gateway`.

To determine which mode your node is running, you can check the 8080/api/v1/roles endpoint. For example:
```
# for http
http://<IP_ADDRESS>:8080/api/v1/roles

# for https reversed proxy
https://<DOMAIN>/api/v1/roles
```

Everything necessary will exist on your node by default. For instance, if you're running a mixnode, you'll find that a NR (Network Requester) and IPR (IP Packet Router) address exist, but they will be ignored in `mixnode` mode.

For more information about available endpoints and their status, you can refer to:
```
# for http
http://<IP>:8080/api/v1/swagger/#/

# for https reversed proxy
https://<DOMAIN>/api/v1/swagger/#/
```


### Flags

There are a few changes from the individual binaries used in the past. For example by default `run` command does `init` function as well, local node `--id` will be set by default unless specified otherwise etcetera.

Run `./nym-node --help` to see all available commands:

~~~admonish example collapsible=true title="`./nym-node --help` output:"
```
<!-- cmdrun ../../../../target/release/nym-node --help -->
```
~~~

To list all available flags for each command, run `./nym-node <COMMAND> --help` for example `./nym-node run --help`:

~~~admonish example collapsible=true title="`./nym-node run --help` output:"
```
<!-- cmdrun ../../../../target/release/nym-node run --help  -->
```
~~~

#### Flag Summary

Some of the most useful flags and their explanation:

- `--id <YOUR_ID>`: Local identifier of your node. <!--config, default? -->
- `--config-file <PATH>`: Used for the migrate command to indicate the location of the existing node config file.
- `--deny-init`: Use this flag to prevent a new node from being initialized. It's recommended to use this after the first run to avoid accidental spinning up of a second node.
- `--init-only`: Use this flag if you want to set up a node without starting it.
- `--mode`: Determines the mode of the node and is always required.
- `--write-changes`: Used to change values within the `config.toml` file after the node has been run.
- `--mnemonic`: This is for when gateways are coconut-credentials-enforced, and this mnemonic is used as the `double_spend` prevention. This account needs credit in order for it to work.
- `--expose-system-info <true/false>`: Sets your system info visibility on the network.
- `--expose-system-hardware <true/false>`: Sets your system hardware info visibility on the network.
- `--expose-crypto-hardware <true/false>`: Sets your crypto hardware info visibility on the network.
