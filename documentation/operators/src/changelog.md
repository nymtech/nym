# Changelog

This page displays a full list of all the changes during our release cycle from [`v2024.3-eclipse`](https://github.com/nymtech/nym/blob/nym-binaries-v2024.3-eclipse/CHANGELOG.md) onwards. Operators can find here the newest updates together with links to relevant documentation. The list is sorted so that the newest changes appear first.


## `v2024.5-ragusa`

- [Release binaries](https://github.com/nymtech/nym/releases/tag/nym-binaries-v2024.5-ragusa)
- [Release CHANGELOG.md](https://github.com/nymtech/nym/blob/nym-binaries-v2024.5-ragusa/CHANGELOG.md)
- [`nym-node`](nodes/nym-node.md) version `1.1.2`
~~~admonish example collapsible=true title="CHANGELOG.md"
- Feature/nym node api location ([#4605])
- Add optional signature to IPR request/response ([#4604])
- Feature/unstable tested nodes endpoint ([#4601])
- nym-api: make report/avg_uptime endpoints ignore blacklist ([#4599])
- removed blocking for coconut in the final epoch state ([#4598])
- allow using explicit admin address for issuing freepasses ([#4595])
- Use rfc3339 for last_polled in described nym-api endpoint ([#4591])
- Explicitly handle constraint unique violation when importing credential ([#4588])
- [bugfix] noop flag for nym-api for nymvisor compatibility ([#4586])
- Chore/additional helpers ([#4585])
- Feature/wasm coconut ([#4584])
- upgraded axum and related deps to the most recent version ([#4573])
- Feature/nyxd scraper pruning ([#4564])
- Run cargo autoinherit on the main workspace ([#4553])
- Add rustls-tls to reqwest in validator-client ([#4552])
- Feature/rewarder voucher issuance ([#4548])

[#4605]: https://github.com/nymtech/nym/pull/4605
[#4604]: https://github.com/nymtech/nym/pull/4604
[#4601]: https://github.com/nymtech/nym/pull/4601
[#4599]: https://github.com/nymtech/nym/pull/4599
[#4598]: https://github.com/nymtech/nym/pull/4598
[#4595]: https://github.com/nymtech/nym/pull/4595
[#4591]: https://github.com/nymtech/nym/pull/4591
[#4588]: https://github.com/nymtech/nym/pull/4588
[#4586]: https://github.com/nymtech/nym/pull/4586
[#4585]: https://github.com/nymtech/nym/pull/4585
[#4584]: https://github.com/nymtech/nym/pull/4584
[#4573]: https://github.com/nymtech/nym/pull/4573
[#4564]: https://github.com/nymtech/nym/pull/4564
[#4553]: https://github.com/nymtech/nym/pull/4553
[#4552]: https://github.com/nymtech/nym/pull/4552
[#4548]: https://github.com/nymtech/nym/pull/4548
~~~

### Features

- New `nym-node` API endpoint `/api/v1/auxiliary-details`: to expose any additional information. Currently it's just the location. `nym-api` will then query all nodes for that information and put it in the `self-described` endpoint.
- Newly `nym-node` location available - use one of the three options to add this to your node config:
    1. Update the `location` field under `[host]` section of `config.toml`
    2. For new nodes: Initialise the node with `--location` flag, where they have to provide the country info. Either full country name (e.g. 'Jamaica'), two-letter alpha2 (e.g. 'JM'), three-letter alpha3 (e.g. 'JAM') or three-digit numeric-3 (e.g. '388') can be provided.
    3. For existing nodes: It's also possible to use exactly the same `--location` argument as above, but make sure to also provide `--write-changes` (or `-w`) flag to persist those changes!
- [Feature/unstable tested nodes endpoint](https://github.com/nymtech/nym/pull/4601): Adds new data structures (`TestNode`, `TestRoute`, `PartialTestResult`) to handle test results for Mixnodes and Gateways. With the inclusion of pagination to handle large API responses efficiently. Lastly, introducing a new route with the tag `unstable` thus meaning not to be consumed without a user risk, prefixes in endpoints with unstable, are what it says on the tin.
    - Testing steps performed:
        - Deploy new api changes to sandbox environment
        - Ensure current operations are transactional and standed operations are working
        - Run a script to ensure that the new endpoints are working as expected with pagination 

 <img width="719" alt="image" src="https://github.com/nymtech/nym/assets/60836166/91285971-e82a-4e5a-8a58-880505ae1be9">

- [`nym-api`: make report/avg_uptime endpoints ignore blacklist](https://github.com/nymtech/nym/pull/4599): When querying for node specific data, it's no longer going to go through the entire list of all cached (and filtered nodes) to find it; instead it will attempt to retrieve a single unfiltered entry.
    - Testing steps performed:
        - Built the project and deployed it in a test environment.
        - Manually tested API endpoints for mixnode and gateway data.
        - Verified that the endpoints return the expected data and handle blacklists correctly.
        - API performance improved due to the efficient `HashMap` lookups
        - Data in mainnet will differ from test nets due to the increased amount of gateways and mixnodes in that environment
        - Test standard uptime routes:
```sh
curl -X 'GET' 'https://validator.nymtech.net/api/v1/status/gateway/Fo4f4SQLdoyoGkFae5TpVhRVoXCF8UiypLVGtGjujVPf/avg_uptime' -H 'accept: application/json'
```


### Documentation Updates

- [`nym-gateway-probe`](testing/gateway-probe.md): A CLI tool to check in-real-time networking status of any Gateway locally.
- [Where to host your `nym-node`?](legal/isp-list.md): A list of Internet Service Providers (ISPs) by Nym Operators community. We invite all operators to add their experiences with different ISPs to strengthen the community knowledge and Nym mixnet performance. 

## `v2024.4-nutella`

- [Merged PRs](https://github.com/nymtech/nym/milestone/59?closed=1)
- [`nym-node`](nodes/nym-node.md) version `1.1.1`
- This release also contains: `nym-gateway` and `nym-network-requester` binaries
- core improvements on nym-node configuration
- Nym wallet changes:
    - Adding `nym-node` command to bonding screens
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
