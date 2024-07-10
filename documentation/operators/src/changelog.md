# Changelog

This page displays a full list of all the changes during our release cycle from [`v2024.3-eclipse`](https://github.com/nymtech/nym/blob/nym-binaries-v2024.3-eclipse/CHANGELOG.md) onwards. Operators can find here the newest updates together with links to relevant documentation. The list is sorted so that the newest changes appear first.

## `v2024.7-doubledecker`

- [Release binaries](https://github.com/nymtech/nym/releases/tag/nym-binaries-v2024.7-doubledecker)
- [Release CHANGELOG.md](https://github.com/nymtech/nym/blob/nym-binaries-v2024.7-doubledecker/CHANGELOG.md)
- [`nym-node`](nodes/nym-node.md) version `1.1.4`

~~~admonish example collapsible=true title='CHANGELOG.md'
- Add an early return in `parse_raw_str_logs` for empty raw log strings. ([#4686])
- Bump braces from 3.0.2 to 3.0.3 in /wasm/mix-fetch/internal-dev ([#4672])
- add expiry returned on import ([#4670])
- [bugfix] missing rustls feature ([#4666])
- Bump ws from 8.13.0 to 8.17.1 in /wasm/client/internal-dev-node ([#4665])
- Bump braces from 3.0.2 to 3.0.3 in /clients/native/examples/js-examples/websocket ([#4663])
- Bump ws from 8.14.2 to 8.17.1 in /sdk/typescript/packages/nodejs-client ([#4662])
- Update setup.md ([#4661])
- New clippy lints ([#4660])
- Bump braces from 3.0.2 to 3.0.3 in /nym-api/tests ([#4659])
- Bump braces from 3.0.2 to 3.0.3 in /docker/typescript_client/upload_contract ([#4658])
- Update vps-setup.md ([#4656])
- Update configuration.md ([#4655])
- Remove old PR template ([#4639])

[#4686]: https://github.com/nymtech/nym/pull/4686
[#4672]: https://github.com/nymtech/nym/pull/4672
[#4670]: https://github.com/nymtech/nym/pull/4670
[#4666]: https://github.com/nymtech/nym/pull/4666
[#4665]: https://github.com/nymtech/nym/pull/4665
[#4663]: https://github.com/nymtech/nym/pull/4663
[#4662]: https://github.com/nymtech/nym/pull/4662
[#4661]: https://github.com/nymtech/nym/pull/4661
[#4660]: https://github.com/nymtech/nym/pull/4660
[#4659]: https://github.com/nymtech/nym/pull/4659
[#4658]: https://github.com/nymtech/nym/pull/4658
[#4656]: https://github.com/nymtech/nym/pull/4656
[#4655]: https://github.com/nymtech/nym/pull/4655
[#4639]: https://github.com/nymtech/nym/pull/4639
~~~

### Features

- [Remove the `nym-mixnode` and `nym-gateway` binaries from the CI upload builds action](https://github.com/nymtech/nym/pull/4693): 
- [Add an early return in `parse_raw_str_logs` for empty raw log strings.](https://github.com/nymtech/nym/pull/4686): This accommodates for the v50 + chain upgrade.
- [Bump braces from `3.0.2` to `3.0.3` in `/wasm/mix-fetch/internal-dev`](https://github.com/nymtech/nym/pull/4672): Version update of [braces](https://github.com/micromatch/braces)
- [Bump braces from `3.0.2` to `3.0.3` in `/clients/native/examples/js-examples/websocket`](https://github.com/nymtech/nym/pull/4663): Version update of [braces](https://github.com/micromatch/braces).
- [Bump braces from `3.0.2` to `3.0.3` in `/nym-api/tests`](https://github.com/nymtech/nym/pull/4659): Version update of [braces](https://github.com/micromatch/braces).
- [Bump braces from `3.0.2` to `3.0.3` in `/docker/typescript_client/upload_contract`](https://github.com/nymtech/nym/pull/4658): Version update of  [braces](https://github.com/micromatch/braces).
- [Bump `ws` from `8.13.0` to `8.17.1` in `/wasm/client/internal-dev-node`](https://github.com/nymtech/nym/pull/4665): Version update of [`ws`](https://github.com/websockets/ws). 
- [Bump `ws` from `8.14.2` to `8.17.1` in `/sdk/typescript/packages/nodejs-client`](https://github.com/nymtech/nym/pull/4662): Version update of [`ws`](https://github.com/websockets/ws). 
- [Add expiry returned on import](https://github.com/nymtech/nym/pull/4670): We need to return the expiry on import for desktop daemon `nym-vpnd`.
- [New clippy lints](https://github.com/nymtech/nym/pull/4660)
- [Remove `nym-connect` directory](https://github.com/nymtech/nym/pull/4643): Since the `nym-vpn` has superseded `nym-connect`, remove `nym-connect` from the repo. 
- [Remove old PR template](https://github.com/nymtech/nym/pull/4639)

### Bugfix

- [missing rustls feature](https://github.com/nymtech/nym/pull/4666): It just happens to work due to `feature-unification`. It should probably have this feature inbuild.

### Operators Guide updates

- [Node description guide](nodes/configuration.md#node-description): Steps to add self-description to `nym-node` and query this information from any node.
- [Web Secure Socket (WSS) guide and reverse proxy update](/nodes/proxy-configuration.md), PR [here](https://github.com/nymtech/nym/pull/4694): A guide to setup `nym-node` in a secure fashion, using WSS via Nginx and Certbot. Landing page (reversed proxy) is updated and simplified.

---

## `v2024.6-chomp`

- [Release binaries](https://github.com/nymtech/nym/releases/tag/nym-binaries-v2024.6-chomp)
- [Release CHANGELOG.md](https://github.com/nymtech/nym/blob/nym-binaries-v2024.6-chomp/CHANGELOG.md)
- [`nym-node`](nodes/nym-node.md) version `1.1.3`
- Standalone `nym-gateway` and `nym-mixnode` binaries are no longer released

~~~admonish example collapsible=true title='CHANGELOG.md'
- Remove additional code as part of Ephemera Purge and SP and contracts ([#4650])
- bugfix: make sure nym-api can handle non-cw2 (or without detailed build info) compliant contracts ([#4648])
- introduced a flag to accept toc and exposed it via self-described API ([#4647])
- bugfix: make sure to return an error on invalid public ip ([#4646])
- Add ci check for PR having an assigned milestone ([#4644])
- Removed ephemera code ([#4642])
- Remove stale peers ([#4640])
- Add generic wg private network routing ([#4636])
- Feature/new node endpoints ([#4635])
- standarised ContractBuildInformation and added it to all contracts ([#4631])
- validate nym-node public ips on startup ([#4630])
- Bump defguard wg ([#4625])
- Fix cargo warnings ([#4624])
- Update kernel peers on peer modification ([#4622])
- Handle v6 and v7 requests in the IPR, but reply with v6 ([#4620])
- fix typo ([#4619])
- Update crypto and rand crates ([#4607])
- Purge name service and service provider directory contracts ([#4603])

[#4650]: https://github.com/nymtech/nym/pull/4650
[#4648]: https://github.com/nymtech/nym/pull/4648
[#4647]: https://github.com/nymtech/nym/pull/4647
[#4646]: https://github.com/nymtech/nym/pull/4646
[#4644]: https://github.com/nymtech/nym/pull/4644
[#4642]: https://github.com/nymtech/nym/pull/4642
[#4640]: https://github.com/nymtech/nym/pull/4640
[#4636]: https://github.com/nymtech/nym/pull/4636
[#4635]: https://github.com/nymtech/nym/pull/4635
[#4631]: https://github.com/nymtech/nym/pull/4631
[#4630]: https://github.com/nymtech/nym/pull/4630
[#4625]: https://github.com/nymtech/nym/pull/4625
[#4624]: https://github.com/nymtech/nym/pull/4624
[#4622]: https://github.com/nymtech/nym/pull/4622
[#4620]: https://github.com/nymtech/nym/pull/4620
[#4619]: https://github.com/nymtech/nym/pull/4619
[#4607]: https://github.com/nymtech/nym/pull/4607
[#4603]: https://github.com/nymtech/nym/pull/4603
~~~

### Features

- [Make embedded NR/IPR ignore performance of the Gateway](https://github.com/nymtech/nym/pull/4671): fixes bug in relation to scoring issue on nym-nodes operating as exit gateways failing to come online.
- [Introduce a flag to accept Operators Terms and Conditions and exposed it via self-described API](https://github.com/nymtech/nym/pull/4647)
~~~admonish example collapsible=true title='Testing steps performed'
- Verify that the `execute` function correctly checks if the `accept_operator_terms` flag is set.
- Test that a warning is displayed when the `accept_operator_terms` flag is not set.
- Confirm that the `NymNode` instance is initialized with `with_accepted_toc(accepted_toc)` when the flag is set.
- Apply the `--accept-toc` flag in the service and confirmed the change by running:
```
curl -s -X 'GET' 'http://18.171.251.41:8080/api/v1/auxiliary-details?output=json' -H 'accept: application/json' | jq .accepted_toc
```
- Verify that the output is `true`.
~~~

- [Rename 'accept-toc' flag and fields into explicit 'accept-operator-terms-and-conditions'](https://github.com/nymtech/nym/pull/4654): makes the `accept-toc` flag more explicit.
- [Validate nym-node public ips on startup](https://github.com/nymtech/nym/pull/4630): makes sure `nym-node` is not run with an empty `public_ips` and that they do not correspond to common misconfigurations like `127.0.0.1` or `0.0.0.0` unless run with `--local` flag.
~~~admonish example collapsible=true title='Testing steps performed'
- Use the latest release/chomp binary with nym-node and input a dodgy ip
<img width="361" alt="image" src="https://github.com/nymtech/nym/assets/60836166/6f2210f9-90ec-48fb-932f-f325c701de09">

- Validation:
<img width="1104" alt="image" src="https://github.com/nymtech/nym/assets/60836166/3bac221f-82f2-44cd-b8c0-6c599b0eb325">
When restarting the node it complains within the service launch file
~~~

- [New node endpoints](https://github.com/nymtech/nym/pull/4635): introduces new endpoints on nym-api (and creates scaffolding for additional ones) for providing **unfiltered** network topology alongside performance score of all nodes.
    - `NymApiTopologyProvider` got modified to use those endpoints alongside (configurable) filtering of nodes with score < 50% (like our current blacklist)
    - Old clients should work as before as no existing endpoint got removed
~~~admonish example collapsible=true title='Testing steps performed'
- Validate that the `skimmed` endpoints are working, keeping in mind that they are unstable. The *full-fat* and *semi-skimmed* have not yet been implemented.
~~~

- [Remove stale peers](https://github.com/nymtech/nym/pull/4640)
- [Removed ephemera code](https://github.com/nymtech/nym/pull/4642)
~~~admonish example collapsible=true title='Testing steps performed'
- Check references to everything named SP and Ephemera and removed any additional references
~~~

- [Remove additional code as part of Ephemera Purge and SP and contracts](https://github.com/nymtech/nym/pull/4650): in line with [#4642](https://github.com/nymtech/nym/pull/4642) and [#4603](https://github.com/nymtech/nym/pull/4603)
~~~admonish example collapsible=true title='Testing steps performed'
- Check references to everything named SP and Ephemera and removed any additional references
~~~

- [Add ci check for PR having an assigned milestone](https://github.com/nymtech/nym/pull/4644): add a CI check for checking that a PR is assigned to a milestone. Can bypassed the check by adding a `no-milestone` label to a PR
~~~admonish example collapsible=true title='Testing steps performed'
- CI complains if no milestone is associated with the an issue.
~~~

- [Bump defguard wireguard](https://github.com/nymtech/nym/pull/4625)
- [Add generic wireguard private network routing](https://github.com/nymtech/nym/pull/4636): as defguard wireguard only allows for peer routing modifications, we will configure the entire wireguard private network to be routed to the wg device. Configuring per peer is also not desirable, as the interface doesn't allow removing routes, so unused ip routing won't be cleaned until gateway restart (and it would also pollute to routing table with a lot of rules when many peers are added).
~~~admonish example collapsible=true title='Testing steps performed'
- This is a part of a bigger ticket, but initial testing has proven to shown that launching nym-nodes (entry and exit gateways) in WG enable mode to be working

*QA will use this template for the other related WG tickets in this release milestone.*
~~~
- [Standarise `ContractBuildInformation` and add it to all contracts](https://github.com/nymtech/nym/pull/4631): Similarly to `cw2`, we're now saving `ContractBuildInformation` under a constant storage key, i.e. `b"contract_build_info"` that standarises the retrieval by nym-api.
    - Also each of our contracts now saves and updates that information upon init and migration.
~~~admonish example collapsible=true title='Testing steps performed'
- Use the latest release/chomp contracts and deploy these to QA
- Use the `nym-api` to query for the results of these new contracts

```sh
 curl -X 'GET' \
   'https://qa-nym-api.qa.nymte.ch/api/v1/network/nym-contracts-detailed' \
   -H 'accept: application/json'
```

- It returns a detailed view of the contracts and which branch they were built from, alongside rust versions and so forth.
<img width="1257" alt="image" src="https://github.com/nymtech/nym/assets/60836166/b5711431-c2f6-44ee-bf02-b17e6c48c5ee">
~~~

- [Update kernel peers on peer modification](https://github.com/nymtech/nym/pull/4622):
~~~admonish example collapsible=true title='Testing steps performed'
- This is a part of a bigger ticket, but initial testing has proven to shown that launching nym-nodes (entry and exit gateways) in WG enable mode to be working.
*QA will use this template for the other related WG tickets in this release milestone.*
~~~

- [Handle v6 and v7 requests in the IPR, but reply with v6](https://github.com/nymtech/nym/pull/4620): teach the IPR to read both v6 and v7 requests, but always reply with v6. This is to prepare for bumping to v7 and signed connect/disconnect messages. Follow up PRs will add
    - Verify signature
    - Send v7 in client with signatures included
- [Purge name service and service provider directory contracts](https://github.com/nymtech/nym/pull/4603): this is a compiler assisted purge of the `nym-name-service` and `nym-service-provider-directory` contracts that were never deployed on mainnet, and will anyhow be superseded by the new mixnode directory that is being worked on.
~~~admonish example collapsible=true title='Testing steps performed'
It works insofar that it compiles, we need to deploy and test this on non-mainnet before merging in

- Purge `nym-name-service` contract
- Purge `nym-name-service-common`
- Purge `nym-service-provider-directory` contract
- Purge `nym-service-provider-directory-common`
- Remove everywhere name-service contract is used
- Remove everywhere sp contract is used

Performed:
- Check references to everything named SP and Ephemera and removed any additional references
~~~

### Crypto

- [Update crypto and rand crates](https://github.com/nymtech/nym/pull/4607): Update sphinx crate to `0.1.1` along with 25519 crates and `rand` crates
~~~admonish example collapsible=true title="Comments"
This PR contains a test failure due to the update [here](https://github.com/nymtech/nym/blob/b4a0487a41375167b2f481c00917b957b9f89789/common/crypto/src/asymmetric/encryption/mod.rs#L353-L358)

- This is due a change in `x25519-dalek` from `1.1.1` to `2`.
- Crypto operations should be identical, but the byte representation has changed (sphinx clamps at creation, x25519 clamps at use). This cannot be changed in the sphinx crate without breaking changes.
- There is a good chance that this failure doesn't impact anything else, but it has to be tested to see.
- A mix of old and new clients with a mix of old and new mixnodes should do
~~~

### Bugfix
- [Make sure nym-api can handle non-cw2 (or without detailed build info) compliant contracts](https://github.com/nymtech/nym/pull/4648): fixes the issue (even if some contracts aren't uploaded on chain it doesn't prohibit the api from working - caveat, the essential vesting and mixnet contract are required)
~~~admonish example collapsible=true title='Testing steps performed'
- Use the latest release/chomp contracts and deploy these to QA
- If the contract was not found, the API would complain of invalid contracts, thus not starting the rest of the operations of the API (network monitor / rewarding etc)

 `Jun 11 16:27:34 qa-v2-nym-api bash[1352642]:  2024-06-11T16:27:34.551Z ERROR nym_api::nym_contract_cache::cache::refresher > Failed to refresh validator cache - Abci query failed with code 6 - address n14y2x8a60knc5jjfeztt84kw8x8l5pwdgnqg256v0p9v4p7t2q6eswxyusw: no such contract: unknown request`
~~~

- [Make sure to return an error on `nym-node` invalid public ip](https://github.com/nymtech/nym/pull/4646): bugfix for [#4630](https://github.com/nymtech/nym/pull/4630) that interestingly hasn't been detected by clippy.
~~~admonish example collapsible=true title='Testing steps performed'
- Use the latest release/chomp binary with nym-node and input a dodgy ip
<img width="361" alt="image" src="https://github.com/nymtech/nym/assets/60836166/6f2210f9-90ec-48fb-932f-f325c701de09">

- Validation:
<img width="1104" alt="image" src="https://github.com/nymtech/nym/assets/60836166/3bac221f-82f2-44cd-b8c0-6c599b0eb325">
~~~

- [Extend the return error when connecting to gateway fails](https://github.com/nymtech/nym/pull/4626)
~~~admonish example collapsible=true title='Testing steps performed'
- Verify that the `establish_connection` function correctly attempts to establish a connection to the gateway.
- Test error handling for `NetworkConnectionFailed` by simulating a failed connection.
- Ensure that the `NetworkConnectionFailed` error includes the `address` and `source` details as expected.
- Checked that `SocketState::Available` is set correctly when a connection is successfully established.
~~~

- [Fix Cargo warnings](https://github.com/nymtech/nym/pull/4624): On every cargo command we have the set warnings:
~~~admonish example collapsible=true title="Cargo warnings"
warning: /home/alice/src/nym/nym/common/dkg/Cargo.toml: `default-features` is ignored for bls12_381, since `default-features` was not specified for `workspace.dependencies.bls12_381`, this could become a hard error in the future warning: /home/alice/src/nym/nym/common/dkg/Cargo.toml: `default-features` is ignored for ff, since `default-features` was not specified for `workspace.dependencies.ff`, this could become a hard error in the future warning: /home/alice/src/nym/nym/common/dkg/Cargo.toml: `default-features` is ignored for group, since `default-features` was not specified for `workspace.dependencies.group`, this could become a hard error in the future warning: /home/alice/src/nym/nym/common/client-libs/validator-client/Cargo.toml: `default-features` is ignored for bip32, since `default-features` was not specified for `workspace.dependencies.bip32`, this could become a hard error in the future warning: /home/alice/src/nym/nym/common/client-libs/validator-client/Cargo.toml: `default-features` is ignored for prost, since `default-features` was not specified for `workspace.dependencies.prost`, this could become a hard error in the future warning: /home/alice/src/nym/nym/common/credentials-interface/Cargo.toml: `default-features` is ignored for bls12_381, since `default-features` was not specified for `workspace.dependencies.bls12_381`, this could become a hard error in the future warning: /home/alice/src/nym/nym/common/credentials/Cargo.toml: `default-features` is ignored for bls12_381, since `default-features` was not specified for `workspace.dependencies.bls12_381`, this could become a hard error in the future warning: /home/alice/src/nym/nym/common/nymcoconut/Cargo.toml: `default-features` is ignored for bls12_381, since `default-features` was not specified for `workspace.dependencies.bls12_381`, this could become a hard error in the future warning: /home/alice/src/nym/nym/common/nymcoconut/Cargo.toml: `default-features` is ignored for ff, since `default-features` was not specified for `workspace.dependencies.ff`, this could become a hard error in the future warning: /home/alice/src/nym/nym/common/nymcoconut/Cargo.toml: `default-features` is ignored for group, since `default-features` was not specified for `workspace.dependencies.group`, this could become a hard error in the future.
~~~
    - This PR adds `default-features = false` to the workspace dependencies to fix these. An alternative way would be to remove `default-features = false` in the crates, but we assume these were put there for a good reason. Also we might have other crates outside of the main workspace that depends on these crates having default features disabled.
    - We also have the warning `warning: profile package spec nym-wasm-sdk in profile release did not match any packages`  which we fix by commenting out the profile settings, since the crate is currently commented out in the workspace crate list.
~~~admonish example collapsible=true title='Testing steps performed'
- All binaries have been built and deployed from this branch and no issues have surfaced.
~~~

### Operators Guide updates

- [New Release Cycle](release-cycle.md) introduced: a transparent release flow, including:
    - New environments
    - Stable testnet
    - [Testnet token faucet](https://nymtech.net/operators/sandbox.html#sandbox-token-faucet)
    - Flow [chart](release-cycle.md#release-flow)
- [Sandbox testnet](sandbox.md) guide: teaching Nym node operators how to run their nodes in Nym Sandbox testnet environment.
- [Terms & Conditions flag](nodes/setup.md#terms--conditions)
- [Node API Check CLI](testing/node-api-check.md)
- [Pruning VPS `syslog` scripts](troubleshooting/vps-isp.md#pruning-logs)
- [Black-xit: Exiting the blacklist](troubleshooting/nodes.md#my-gateway-is-blacklisted)

---

## `v2024.5-ragusa`

- [Release binaries](https://github.com/nymtech/nym/releases/tag/nym-binaries-v2024.5-ragusa)
- [Release CHANGELOG.md](https://github.com/nymtech/nym/blob/nym-binaries-v2024.5-ragusa/CHANGELOG.md)
- [`nym-node`](nodes/nym-node.md) version `1.1.2`
~~~admonish example collapsible=true title='CHANGELOG.md'
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
- New `nym-node` location available - use one of the three options to add this to your node config:
    1. Update the `location` field under `[host]` section of `config.toml`
    2. For new nodes: Initialise the node with `--location` flag, where they have to provide the country info. Either full country name (e.g. 'Jamaica'), two-letter alpha2 (e.g. 'JM'), three-letter alpha3 (e.g. 'JAM') or three-digit numeric-3 (e.g. '388') can be provided.
    3. For existing nodes: It's also possible to use exactly the same `--location` argument as above, but make sure to also provide `--write-changes` (or `-w`) flag to persist those changes!
- [Feature/unstable tested nodes endpoint](https://github.com/nymtech/nym/pull/4601): Adds new data structures (`TestNode`, `TestRoute`, `PartialTestResult`) to handle test results for Mixnodes and Gateways. With the inclusion of pagination to handle large API responses efficiently. Lastly, introducing a new route with the tag `unstable` thus meaning not to be consumed without a user risk, prefixes in endpoints with unstable, are what it says on the tin.
~~~admonish example collapsible=true title='Testing steps performed'
- Deploy new api changes to sandbox environment
- Ensure current operations are transactional and standed operations are working
- Run a script to ensure that the new endpoints are working as expected with pagination
 <img width="719" alt="image" src="https://github.com/nymtech/nym/assets/60836166/91285971-e82a-4e5a-8a58-880505ae1be9">
~~~

- [`nym-api`: make report/avg_uptime endpoints ignore blacklist](https://github.com/nymtech/nym/pull/4599): When querying for node specific data, it's no longer going to go through the entire list of all cached (and filtered nodes) to find it; instead it will attempt to retrieve a single unfiltered entry.
~~~admonish example collapsible=true title='Testing steps performed'
- Build the project and deployed it in a test environment.
- Manually test API endpoints for mixnode and gateway data.
- Verify that the endpoints return the expected data and handle blacklists correctly.
- API performance improved due to the efficient `HashMap` lookups
- Data in mainnet will differ from test nets due to the increased amount of gateways and mixnodes in that environment
- Test standard uptime routes:
```sh
curl -X 'GET' 'https://validator.nymtech.net/api/v1/status/gateway/Fo4f4SQLdoyoGkFae5TpVhRVoXCF8UiypLVGtGjujVPf/avg_uptime' -H 'accept: application/json'
```
~~~

- [Use rfc3339 for last_polled in described nym-api endpoint](https://github.com/nymtech/nym/pull/4591): Fix issue where the validator-client can't parse the nym-api response for the described endpoint, in particular the `latest_polled` field that was recently added, by making the field use `rfc3339`
    - **Note:** This will require upgrading `nym-api` and everything that depends on the described endpoint.
~~~admonish example collapsible=true title='Testing steps performed'
- Update a `nym-api` to the binary built from this branch, then restart the api
- Check the `journalctl` for error messages
- Connected via client and could not see the error messages, this is backwards compatible
- Local testing using sdk examples:
```sh
cd <PATH_TO>/nym/sdk/rust/nym-sdk
cargo run --example simple

# outcome
thread 'main' panicked at sdk/rust/nym-sdk/examples/simple.rs:9:64:
called Result::unwrap() on an Err value: ClientCoreError(ValidatorClientError(NymAPIError { source: ReqwestClientError { source: reqwest::Error { kind: Request, url: Url { scheme: "https", cannot_be_a_base: false, username: "", password: None,
```
~~~

- [Upgrade `axum` and related dependencies to the most recent version](https://github.com/nymtech/nym/pull/4573)
- [Run cargo autoinherit on the main workspace](https://github.com/nymtech/nym/pull/4553): Move several dependencies to the workspace level using cargo autoinherit, to make it easier to keep our dependencies up to date.
    - Run cargo autoinherit in the root
    - Merge in the new workspace deps in the main list
    - We made sure to not mix in other changes as well - all features flags for all crates should be the same as before
~~~admonish example collapsible=true title='Testing steps performed'
- Run `cargo autoinherit` in the root directory to move dependencies to the workspace level
- Merge the new workspace dependencies into the main list
- Ensure no other changes were mixed in during the process
- Verify that all feature flags for all crates remained the same as before
- Build all the binaries from this branch to confirm successful compilation
- Deploy the built binaries across different environments to ensure there were no issues
~~~

- [Add rustls-tls to reqwest in validator-client](https://github.com/nymtech/nym/pull/4552): An attempt to make possible to end up in a situation where use use the validator-client but without functioning TLS support. For the monorepo this is masked by cargo feature unification, but becomes a problem for outside consumers, as as been noticed in many of the vpn client implementations.
    - In `validator-client`: `reqwest`, enable `rustls-tls` for `non-wasm32`
    - In `client-core`: Use default features enabled for `non-wasm32` and switch to `webpki` roots, since that's what we're using with `reqwest` anyway
    - In `gateway-client`: Switch to `webpki` roots, since that's what we're using with `reqwest` anyway

#### Crypto

- [Remove blocking for coconut in the final epoch state](https://github.com/nymtech/nym/pull/4598)
~~~admonish example collapsible=true title='Testing steps performed'
- Build the project to ensure no compilation errors
- Run tests to verify the functionality of the `issue_credential` function
- Execute integration tests to check the behaviour during an epoch transition.
~~~

- [Allow using explicit admin address for issuing freepasses](https://github.com/nymtech/nym/pull/4595)
- [Explicitly handle constraint unique violation when importing credential](https://github.com/nymtech/nym/pull/4588): Add a strong type for when a duplicate credential is imported so the vpn lib can handle this.
- [Feature/wasm coconut](https://github.com/nymtech/nym/pull/4584): This pull request requires [\#4585](https://github.com/nymtech/nym/pull/4585) to be merged first
- [Feature/nyxd scraper pruning](https://github.com/nymtech/nym/pull/4564): This PR introduces storage pruning to `nyxd` scraper which is then used by the validators rewarder.
~~~admonish example collapsible=true title='Testing steps performed'
- Add a `main.rs` file in the `nyxd` scraper dir, underneath `lib.rs`, amend `config.pruning_options.validate()?;` to be `let _ = config.pruning_options.validate();` in the mod.rs file
- Test the different variations of `pruning_options`:
    - Check the *default* option: `pruning_options: PruningOptions::default()`
    - Check the *nothing* option: `pruning_options: PruningOptions::nothing()`
    - Check the *custom* option, example: `pruning_options: PruningOptions { keep_recent: (500), interval: (10), strategy: (PruningStrategy::Custom) }`
    - Check the pruning *in real life* for the validator rewarder
- Validate that the database table `blocks` was being updated accordingly
~~~

- [Feature/rewarder voucher issuance](https://github.com/nymtech/nym/pull/4548)
    - Introduces signature checks on issued credential data
    - Stores evidence of any failures/malicious behaviour in the internal db

### Bugfix

- [`noop` flag for `nym-api` for `nymvisor` compatibility](https://github.com/nymtech/nym/pull/4586)
    - The application starts correctly and logs the starting message
    - The `--no_banner` flag works as intended, providing compatibility with `nymvisor`
~~~admonish example collapsible=true title='Testing steps performed'
- Build the project to ensure no compilation errors
- Run the binary with different command-line arguments to verify the CLI functionality
- Test with and without the `--no_banner` flag to ensure compatibility and expected behavior
- Verify logging setup and configuration file parsing
~~~

### Operators Guide updates

- [`nym-gateway-probe`](testing/gateway-probe.md): A CLI tool to check in-real-time networking status of any Gateway locally.
- [Where to host your `nym-node`?](legal/isp-list.md): A list of Internet Service Providers (ISPs) by Nym Operators community. We invite all operators to add their experiences with different ISPs to strengthen the community knowledge and Nym mixnet performance.
- Make sure you run `nym-node` with `--wireguard-enabled false` and add a location description to your `config.toml`, both documented in [`nym-node` setup manual](nodes/setup.md#mode-exit-gateway).


---

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


---

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
