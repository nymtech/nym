<!--If this page was ever to be uncomented, please keep in mind that nym connect is no longer existing -->

# Version Compatibility Table


There are numerous components to Nym which are released independently of one another aside from when breaking changes occur in the [core platform code](https://github.com/nymtech/nym/) - mix nodes, gateways, network requesters, and clients.

Whilst in general it recommended to be running the most recent version of any software, if you cannot do that for whatever reason this table will tell you which versions of different components are mutually compatible with which platform code releases.


| Core Platform    | SDK           | Wallet          | NymConnect      | Network Explorer | Mixnet contract | Vesting contract |
| ---------------- | ------------- | --------------- | --------------- | ---------------- | --------------- | ---------------- |
| 1.1.13 - 1.1.14* | 1.1.7         | 1.1.12 - 1.1.13 | 1.1.12 - 1.1.13 | 1.1.2            | 1.2.0 - 1.3.0   | 1.2.0 - 1.3.0    |
| 1.1.1 - 1.1.12   | 1.1.4 - 1.1.7 | 1.1.0 - 1.1.12  | 1.1.1 - 1.1.12  | 1.1.0 - 1.1.2    | 1.1.0 - 1.1.3   | 1.1.0  - 1.1.3   |
| 1.1.0 - 1.1.1    | 1.1.4         | 1.1.0           | 1.1.0 - 1.1.1   | 1.1.0            | 1.1.0           | 1.1.0            |
| 1.1.0            | x             | 1.1.0           | 1.1.0           | 1.1.0            | 1.1.0           | 1.1.0            |

`*` the `nym-mixnode` binary is currently one point ahead of the main platform release version

> There are seperate changelogs for [`NymConnect`](https://github.com/nymtech/nym/blob/release/{{platform_release_version}}/nym-connect/CHANGELOG.md) and the [`Desktop Wallet`](https://github.com/nymtech/nym/blob/release/{{platform_release_version}}/nym-wallet/CHANGELOG.md). The changelog referenced below is for the core platform code.

| Platform release changelog                                                               |
| ---------------------------------------------------------------------------------------- |
| 1.1.14 ([CHANGELOG](https://github.com/nymtech/nym/blob/release/v1.1.14/CHANGELOG.md))   |
| 1.1.13 ([CHANGELOG](https://github.com/nymtech/nym/blob/release/v1.1.13/CHANGELOG.md))   |
| 1.1.12 ([CHANGELOG](https://github.com/nymtech/nym/blob/release/v1.1.12/CHANGELOG.md))   |
| 1.1.11 ([CHANGELOG](https://github.com/nymtech/nym/blob/release/v1.1.11/CHANGELOG.md))   |
| 1.1.10 ([CHANGELOG](https://github.com/nymtech/nym/blob/release/v1.1.10/CHANGELOG.md))   |
| 1.1.9 ([CHANGELOG](https://github.com/nymtech/nym/blob/release/v1.1.9/CHANGELOG.md))     |
| 1.1.8 ([CHANGELOG](https://github.com/nymtech/nym/blob/release/v1.1.8/CHANGELOG.md))     |
| 1.1.7 ([CHANGELOG](https://github.com/nymtech/nym/blob/release/v1.1.7/CHANGELOG.md))     |
| 1.1.6 ([CHANGELOG](https://github.com/nymtech/nym/blob/release/v1.1.6/CHANGELOG.md))     |
| 1.1.5 ([CHANGELOG](https://github.com/nymtech/nym/blob/release/v1.1.5/CHANGELOG.md))     |
| 1.1.4 ([CHANGELOG](https://github.com/nymtech/nym/blob/release/v1.1.4/CHANGELOG.md))     |
| 1.1.3 ([CHANGELOG](https://github.com/nymtech/nym/blob/release/v1.1.3/CHANGELOG.md))     |
| 1.1.2 ([CHANGELOG](https://github.com/nymtech/nym/blob/release/v1.1.2/CHANGELOG.md))     |
| 1.1.1 ([CHANGELOG](https://github.com/nymtech/nym/blob/release/v1.1.1/CHANGELOG.md))     |
| 1.1.0 ([CHANGELOG](https://github.com/nymtech/nym/blob/release/v1.1.0/CHANGELOG.md))     |
| 1.0.2 ([CHANGELOG](https://github.com/nymtech/nym/blob/nym-binaries-1.0.2/CHANGELOG.md)) |
| 1.0.1 ([CHANGELOG](https://github.com/nymtech/nym/blob/nym-binaries-1.0.1/CHANGELOG.md)) |
| 1.0.0 ([CHANGELOG](https://github.com/nymtech/nym/blob/nym-binaries-1.0.0/CHANGELOG.md)) |
