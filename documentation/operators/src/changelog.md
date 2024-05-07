# Changelog

This page displays a full list of all the changes during our release cycle from [`v2024.3-eclipse`](https://github.com/nymtech/nym/blob/nym-binaries-v2024.3-eclipse/CHANGELOG.md) onwards. Operators can find here the newest updates together with links to relevant documentation. The list is sorted so that the newest changes appear first.

## `v2024.4-nutella`

- [Merged PRs](https://github.com/nymtech/nym/milestone/59?closed=1)
- [`nym-node`](nodes/nym-node.md) version `1.1.1`
- This release also contains: `nym-gateway` and `nym-network-requester`
- core improvements on nym-node configuration
- Nym wallet changes:
    - Adding the `nym-node` command to bonding screens
    - Fixed the delegation issues with fixing RPC
- [Network configuration](nodes/configuration.md#connectivity-test-and-configuration) section updates, in particular for `--mode mixnode` operators
- [VPS IPv6 troubleshooting](troubleshooting/vps-isp.md#ipv6-troubleshooting) updates

## `v2024.3-eclipse`

- Release [Changelog.md](https://github.com/nymtech/nym/blob/nym-binaries-v2024.3-eclipse/CHANGELOG.md)
- [`nym-node`](nodes/nym-node.md) initial release
- New tool for monitoring Gateways performance [harbourmaster.nymtech.net](https://harbourmaster.nymtech.net)
- New versioning `1.1.0+nymnode` mainly for internal migration testing, not essential for operational use. We aim to correct this in a future release to ensure mixnodes feature correctly in the main API
- New [VPS specs & configuration](nodes/vps-setup.md) page
- New [configuration page](nodes/configuration.md) with [connectivity setup guide](nodes/configuration.md#connectivity-test-and-configuration) - a new requirement for `exit-gateway`
- API endpoints redirection: Nym-mixnode and nym-gateway endpoints will eventually be deprecated; due to this, their endpoints will be redirected to new routes once the `nym-node` has been migrated and is running

**API endpoints redirection**

| Previous endpoint              | New endpoint                             |
| ---                            | ---                                      |
| `http://<IP>:8000/stats`       | `http://<IP>:8000/api/v1/metrics/mixing` |
| `http://<IP>:8000/hardware`    | `http://<IP>:8000/api/v1/system-info`    |
| `http://<IP>:8000/description` | `http://<IP>:8000/api/v1/description`    |
