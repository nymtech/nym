## Localnet:

Result of marrying the dedicated `localnet.sh` script and the old `Testnet Manager`.
It allows to run a complete Nym mixnet test environment on Apple's `container` runtime or on Linux `containerd` (via
`nerdctl` and kata shim).

It results in creation of the following containers:

- `nyxd`
- `nym-api`
- `nym-node-1` (gateway)
- `nym-node-2` (mixnode)
- `nym-node-3` (mixnode)
- `nym-node-4` (mixnode)

which run on a custom brige network (`nym-localnet-network`) with dynamic IP assignment:

```
Host Machine (macOS)
├── nym-localnet-network (bridge)
│   ├── nyxd            (192.168.66.3)
│   ├── nym-api         (192.168.66.4)
│   ├── nym-node-1      (192.168.66.5)
│   ├── nym-node-2      (192.168.66.6)
│   ├── nym-node-3      (192.168.66.7)
│   └── nym-node-4      (192.168.66.8)
```

it also embeddeds `nym-gateway-probe` binary in the container image for easy testing.

### Prerequisites

#### MacOS

- **MUST** have MacOS Tahoe for inter-container networking
- `brew install --cask container`
- Download Kata Containers 3.20, this one can be loaded by `container` and has `CONFIG_TUN=y` kernel flag
    - `https://github.com/kata-containers/kata-containers/releases/download/3.20.0/kata-static-3.20.0-arm64.tar.xz`
- Load new kernel
    -
    `container system kernel set --tar kata-static-3.20.0-arm64.tar.xz --binary opt/kata/share/kata-containers/vmlinux-6.12.42-162`
- Validate kernel version once you have container running

#### Linux

The following dependencies must be installed:

- `newuidmap` and `newgidmap` which can be installed via `uidmap` package
- `containerd` which will probably come with your distro
- `nerdctl`, `kata-runtime` and `containerd-shim-kata-v2`. they can be either installed manually or via
  `kata-manager.sh` script: https://github.com/kata-containers/kata-containers/blob/main/utils/README.md#kata-manager.
  it is recommended to run it with the `-N` flag to install it alongside `nerdctl`

### Quick Start

```bash
# navigate to the root of the nym monorepo
# (exact command will depend on the relative location of the directory on your machine)
cd nym

# build the orchestrator binary
cargo run --release --bin localnet-orchestrator

# run the orchestrator binary to startup the network
target/release/nym-localnet-orchestrator up

# run the gateway probe test
target/release/nym-localnet-orchestrator run-gateway-probe-test

# purge all the containers and build data
target/release/nym-localnet-orchestrator purge
```

### Startup flow

The startup is separated into 4 main steps (which can also be run individually as separate commands)

1. `initialise-nyxd`
    - builds `nyxd` docker image from https://github.com/nymtech/nyxd.git and imports it into the `container` runtime
    - initialises the `genesis.json` of the localnet chain and saves it to a shared volume
    - starts up `nyxd` container using the shared volume data

2. `initialise-contracts`
    - either downloads nym contracts or builds all of them fresh using `cosmwasm/optimizer` image
    - uploads and initialises all the contracts onto the chain
    - fixes up state inconsistencies (the bootstrap problem) by performing additional contract migrations

3. `initialise-nym-api`
    - builds `nym-binaries` docker image and imports it into the `container` runtime. note: its version tag is based on
      the current version of the `nym-node` binary
    - generates DKG keys to allow future zk-nym issuance and injects those into a shared volume to be used by the
      `nym-api`
    - initialises `nym-api` data and starts its container using a shared volume
    - overwrites the states of the `dkg` and `group` contracts by forcing the just created `nym-api` instance to be a
      valid zk-nym issuer

4. `initialise-nym-nodes`
    - initialises data of 4 nym-nodes `nym-node --init-only`: 3 mixnodes and 1 gateway
    - bonds all of them into previously created mixnet contract
    - force assigns them to the active set by performing additional admin-only contract shenanigans
    - force refreshes nym-api caches to make the nodes appear in the relevant endpoints immediately
    - injects fake "100%" network monitor scores for each node in the `nym-api` container to make sure all nodes have
      valid performance metrics and force refreshes the relevant cache

### Commands

#### `build-info`

Show build information of the localnet orchestrator binary

#### `initialise-nyxd`

Initialise new nyxd instance as described above

##### Relevant arguments:

- `nyxd-tag` to allow using non-default nyxd repo branch

#### `initialise-contracts`

Upload and initialise all Nym cosmwasm contracts as described above

##### Relevant arguments:

- `monorepo-root` - specify path to the monorepo root if the current working directory is different from the root
- `reproducible-builds` - ensure contract builds are fully reproducible by removing additional source of
  non-determinism. note that this slows down the build process significantly
- `ci-build-branch` - use prebuilt contracts from the `build.ci.nymte.ch` server
- `cosmwasm-optimizer-image` - cosmwasm optimizer image used for building and optimising the contracts
- `allow-cached-build` - allow using pre-built contracts from previous localnet runs

#### `initialise-nym-api`

Initialise instance of nym api and adjust the DKG contract to allow it to immediately start issuing zk-nyms as described
above

##### Relevant arguments:

- `monorepo-root` - specify path to the monorepo root if the current working directory is different from the root
- `cosmwasm-optimizer-image` - cosmwasm optimizer image used for building and optimising the contracts
- `allow-cached-build` - allow using pre-built contracts from previous localnet runs
- `custom-dns` - allows specifying custom nameserver to be used by all spawned containers

#### `initialise-nym-nodes`

Initialise nym nodes to start serving mixnet (and wireguard) traffic. this involves bonding them in the contract and
starting the containers as described above

##### Relevant arguments:

- `monorepo-root` - specify path to the monorepo root if the current working directory is different from the root
- `open-proxy` - allow internal service providers to run in open proxy mode
- `custom-dns` - allows specifying custom nameserver to be used by all spawned containers

#### `run-gateway-probe-test`

Run a gateway probe against the running localnet

##### Relevant arguments:

- `monorepo-root` - specify path to the monorepo root if the current working directory is different from the root
- `prove-args` - allows specifying additional flags to be passed to the gateway probe

#### `rebuild-binaries-image`

Rebuild the docker and container image used for running the nym binaries

##### Relevant arguments:

- `monorepo-root` - specify path to the monorepo root if the current working directory is different from the root
- `custom-tag` - custom image tag for the new image

#### `up`

Single command to start up localnet with minimal configuration

##### Relevant arguments:

refer to arguments of `initialise-nyxd`, `initialise-contracts`, `initialise-nym-api` and `initialise-nym-nodes` as the
same ones are available

#### `down`

Stop the localnet (stops and removes all containers using `localnet-*` image

#### `purge`

Remove all localnet information, including any containers and images

##### Relevant arguments:

- `monorepo-root` - specify path to the monorepo root if the current working directory is different from the root
- `remove-cache` (default: true) - specify whether the cache data should be removed
- `remove-images` (default: true) - specify whether the built images should be removed

### Storage

All the localnet data is saved, by default, under `~/.nym/localnet-orchestrator/` directory and further split into the
following:

- `network-data.sqlite` (by default `~/.nym/localnet-orchestrator/network-data.sqlite`) which contains basic network
  metadata - it was easier than jugling random .json files around
- each container has its volume stored in:
    - $NETWORK_NAME/nym-api (e.g. `~/.nym/localnet-orchestrator/group-key/nym-api`)
    - $NETWORK_NAME/nyxd (e.g. `~/.nym/localnet-orchestrator/group-key/nyxd`)
    - $NETWORK_NAME/nym-node-1 (e.g. `~/.nym/localnet-orchestrator/group-key/nym-node-1`)
    - $NETWORK_NAME/nym-node-2 (e.g. `~/.nym/localnet-orchestrator/group-key/nym-node-2`)
    - $NETWORK_NAME/nym-node-3 (e.g. `~/.nym/localnet-orchestrator/group-key/nym-node-3`)
    - $NETWORK_NAME/nym-node-4 (e.g. `~/.nym/localnet-orchestrator/group-key/nym-node-4`)
- `~/.nym/localnet-orchestrator/.cache` which contains intermediate build data that can be reused between runs to speed
  up the deployment process. currently it only contains `contracts` directory for built cosmwasm contracts

### Current Limitations:

- `nyxd` instance exposes port `26657` to the host. this was to speed up development to allow easier chain interaction
  by being able to use rust client directly from the orchestrator host. in the future this should get modified
- no windows support
- no docker compose - custom orchestrator is used instead
- dynamic ips - container ip addresses may change between restarts, thus there's a lot of inflexibility with a network
  setup. once created it cannot be modified

