# Changelog

This page displays a full list of all the changes during our release cycle from [`v2024.3-eclipse`](https://github.com/nymtech/nym/blob/nym-binaries-v2024.3-eclipse/CHANGELOG.md) onwards. Operators can find here the newest updates together with links to relevant documentation. The list is sorted so that the newest changes appear first.

## `v2024.11-aero`

- [Release binaries](https://github.com/nymtech/nym/releases/tag/nym-binaries-v2024.12-aero)
- [Release CHANGELOG.md](https://github.com/nymtech/nym/blob/nym-binaries-v2024.12-aero/CHANGELOG.md)
- [`nym-node`](nodes/nym-node.md) version `1.1.9`

```sh
nym-node 
Binary Name:        nym-node
Build Timestamp:    2024-10-17T08:57:52.525093253Z
Build Version:      1.1.9
Commit SHA:         d75c7eaaaf3bb7350720cf9c7657ce3f7ee6ec2e
Commit Date:        2024-10-17T08:51:39.000000000+02:00
Commit Branch:      HEAD
rustc Version:      1.81.0
rustc Channel:      stable
cargo Profile:      release
```

### Features

- [Rust sdk stream abstraction](https://github.com/nymtech/nym/pull/4743): Starting to move this from being standalone binaries (as seen [here](https://github.com/nymtech/nym-zcash-grpc-demo)) into the sdk. EDIT this has sort of expanded a bit to include a few things:
  - [x] simple example
  - [x] example doc to `src/tcp_proxy.rs` 
  - [x] simple echo server in `tools/`
  - [x] multithread example
  - [x] example to sdk for using different network
  - [x] go ffi for proxies
  
- [Build(deps): bump `toml` from `0.5.11` to `0.8.14`](https://github.com/nymtech/nym/pull/4805): [`toml`](https://github.com/toml-rs/toml) version update
~~~admonish example collapsible=true title='Testing steps performed'
- Ensured that the `cargo.toml` is legible in various places; tested it on `nym-node`, `nym-api` and `nymvisor`.
- Ensured that updating the cargo.toml file and restarting the given binary continues to behave as normal.
~~~

- [Use `serde` from workspace](https://github.com/nymtech/nym/pull/4833): cargo autoinherit for `serde` - cargo autoinherit for `bs58` and `vergen` in `cosmwasm-smart-contracts` 

- [Gateway database modifications for different modes](https://github.com/nymtech/nym/pull/4868): As gateway clients will not be solely from the mixnet, we need to split the table that handles shared keys from the client ids that are referenced from other tables. That way, the bandwidth table can be shared between different client types (entry mixnet, entry gateway, exit gateway), using the same `client_id` referencing.

- [Remove the push trigger for `ci-nym-wallet-rust`](https://github.com/nymtech/nym/pull/4869)

- [Chore: remove queued migration for adding explicit admin](https://github.com/nymtech/nym/pull/4871)

- [Allow clients to send stateless gateway requests without prior registration](https://github.com/nymtech/nym/pull/4873): in order to make changes to the registration/authentication procedure we needed a way of extracting protocol information before undergoing the handshake. 

- [Fix sql `serde` with `enum`](https://github.com/nymtech/nym/pull/4875)

- [Few fixes to NNM pre deploy](https://github.com/nymtech/nym/pull/4883)

- [Feature/updated gateway registration](https://github.com/nymtech/nym/pull/4885): This PR introduces support for aes256-gcm-siv shared keys between clients and gateways.
    - Those changes should be fully backwards compatible. if they're not, there's a bug.
~~~admonish example collapsible=true title='Testing steps performed'
- For the following combinations I inited the client, ran the client, stopped the client, and ran the client again:
- Fresh client on new binary && gateway on old binary
- Fresh client on old binary && gateway on new binary
- Fresh client on new binary && gateway new binary 
- Existing old client on old binary && new gateway 
~~~
 
- [Build and Push CI](https://github.com/nymtech/nym/pull/4887)

- [Entry wireguard tickets](https://github.com/nymtech/nym/pull/4888): Note: The behaviour of the nodes and vpn client (as a test) has not changed, it still works as it used to. Obtaining ticketbooks also is unchanged

- [Update `nym-vpn` metapackage and replace `nymvpn-x` with `nym-vpn-app`](https://github.com/nymtech/nym/pull/4889): Change dependency from `nymvpn-x` to `nym-vpn-app` to reflect the new package name of the tauri client

- [Update network monitor entry point](https://github.com/nymtech/nym/pull/4893)

- [Remove clippy github PR annotations](https://github.com/nymtech/nym/pull/4896): It eats up CI resources and time to run the clippy annotation checks that likely no one uses anyway. We keep the clippy checks of course.

- [Fix clippy for beta toolchain](https://github.com/nymtech/nym/pull/4897): 

- [Update cargo deny](https://github.com/nymtech/nym/pull/4901): Update to use latest `cargo-deny`.
  - Regenerate `deny.toml`
  - Backport old settings to `deny.toml`
  - Explicitly allow GPL-3 only on our own specific crates
  - Update `deny.toml` for latest changes
  - Fix `cargo-deny` warnings for duplicate crates
  - Update `cargo-deny-action` to v2

- [Data Observatory stub](https://github.com/nymtech/nym/pull/4905): You need Postgres up for `sqlx` compile-time checked queries to work
~~~admonish example collapsible=true title='Try yourself'
```bash
./pg_up.sh
```

Play with the database:
```bash
docker exec -it nym-data-observatory-pg /bin/bash
psql -U youruser -d yourdb
```
~~~

- [Proxy ffi](https://github.com/nymtech/nym/pull/4906): Updates Go & CPP FFI with the proxy code from  [\#4743](https://github.com/nymtech/nym/pull/4743)

- [Bump `http-api-client` default timeout to 30 sec](https://github.com/nymtech/nym/pull/4917)

- [Check both version and type in message header](https://github.com/nymtech/nym/pull/4918)

- [Fix argument to `cargo-deny` action](https://github.com/nymtech/nym/pull/4922)

- [Expose error type](https://github.com/nymtech/nym/pull/4924)

- [Make ip-packet-request VERSION pub](https://github.com/nymtech/nym/pull/4925) 

- [Assume offline mode](https://github.com/nymtech/nym/pull/4926)

- [`nym-node`: don't use bloomfilters for double spending checks](https://github.com/nymtech/nym/pull/4960): this PR disables gateways polling for double spending bloomfilters and also `nym-apis` from providing this data.

### Bugfix

- [Fix `apt install` in `ci-build-upload-binaries.yml`](https://github.com/nymtech/nym/pull/4894)

- [Fix missing duplication of modified tables](https://github.com/nymtech/nym/pull/4904)

- [Fix nymvpn.com url in mainnet defaults](https://github.com/nymtech/nym/pull/4920): The old URL (nympvn.net) works since it is redirected to nymvpn.com, but the extra roundtrip adds latency to all the API calls the vpn client does. So this PR should help speed things up, in particular when these API calls happen across the mixnet.

- [Fix handle drop](https://github.com/nymtech/nym/pull/4934)

- [Replace unreachable macro with an error return](https://github.com/nymtech/nym/pull/4958)

### Operators Guide, Tooling & Updates

#### Documentation Updates

- [Update FAQ sphinx size](https://github.com/nymtech/nym/pull/4946): This PR upgrades url to our code base sphinx creation from an outdated branch to develop. 

#### Fast & Furious - WireGuard edition

Nym team started another round of load and speed testing. This time the tests are limited to Wireguard mode Gateways - to find out any weak spots for needed improvement. The load testing is happening directly on mainnet as it simulates a real user traffic which the network components must be able to handle in order.

Over past week we ran a total of three tests, with 450 clients at most. We've managed to push around 300 GB in total. Around 50% of requests failed. Over the course of those three tests, we did about 5000 requests, and bandwidth per client varies between 50Mb/s and 150Mb/s.

We already caught two bugs and [fixed](https://github.com/nymtech/nym/pull/4885) it in this release. 

**The faster the operators upgrade to this [latest release](https://github.com/nymtech/nym/releases/tag/nym-binaries-v2024.12-aero), the better**. A that will allow us to do more precise testing through the nodes without the registry bug, leading to more precise specs for `nym-node`.

Here are the aims of these tests:

1. Understanding of the wireguard network behavior under full load
    - How many client users can all entry gateways and exit gateways handle simultaneously?
    - How much sustained IP traffic can a subset of mainnet nodes sustain?
2. Needed improvements of Nym Node binaries to improve the throughput on mainnet
3. Measurement of required machine specs
    - Releasing a new spec requirements
4. Raw data record
5. Increase quality of Nym Nodes

Meanwhile we started to research pricing of stronger servers with unlimited bandwidth and higher (and stable) port speed, to arrive to a better understanding of needed rewards and grants to bootstrap the network before NymVPN launch. 

More info about testing and tools for performance monitoring can be found in [this chapter](testing/performance.md).

> We would like to call out to operators to join the efforts and reach out to us if they know of solid ISPs who offer reliable dedicated services for good price or may even be interested in partnership.

#### Delegation Program

In October we again proceeded with our Delegation Program. 22 nodes didn't meet the program rules and got their delegation removed and 25 nodes from the que received delegation. Below is a complete list. 

~~~admonish example collapsible=true title='List of all delegation changes'
Delegated:
```
Ce6kcPckNfQsga2z645VFQYadtoTjqXrS1YXMTtNNv98
2XSCWy1vAoJRaYBJXx4KWwjU1cfoS2wNBXVQZvi8Jtdr
Bu4sUGjJqkje4vSncTH2KgrnojmfESdaYwamC6DbpJGZ
7TWEw9qQxsc8w4WhPAX6zjZ8vuNBdtP21zUVN8K26RkD
HejyqervmGTCEwi1JbRBXV5My463336huBn8ZgSpuhc3
CXcCVGiamYSwgVwaxW3mEkXkZh1sKY2TXnWjjTjxDxzA
FScLfnKUPv9wSef3R4N2pQ9ft7DiwdivLW1i65Dqfc9L
2vuZZJjyYN27fvDbhyqeGosewGWaRh6iVsFtqbJoYAR7
B9QiBsSAx7MRcTpYMs1fu9AFJurAZTPWMispHZXPbaVW
E3e2a9kXZjQXsKAfvmCf2WqwmVkiGR2LbjCwoadZgEJt
Dk4fCLM7idHPqfsUucLQtSMtYaYCLhi4T7vwvw88jG3P
9xZUp4sYWUNJesWy3MPVjh5kTorNqj3RxcFgBmYjV1xV
HK9QxPpdJfNtNpLJZHTN5M113jeBbFzTkMtPt9eouimx
ECkzyHfoiNGKyDTtbbH5HDCWa8KMGh92mtGbGHLZ3Y9n
9jQQV9vQ2mFFXywwVhACCKefjUFpyBoCU6KXNfjAEi45
6QguhCfnDPKJe8bQXg9myuPB89yYFk6R77vMhLTbipK7
4hAJJQhLTFve8FZGd28ksjavbch8STMax2rytzKmDPCV
EZLFq5HGXFKRpxu78nVjf7kuuUaKPLAbezR6mXbZrP6y
FtAAA5GMxY1Ge9wKYDrQgaSfJEUp4XvBLptBwy3GU8ap
tUiLPjz5nkPn5ZJT5ZXLPGDcZ3caQsfkMAp1epoAuSQ
4ScsM6AVowhKTMWaH98NLntKDwbu2ZMEycUk4mZiZppG
Hb34PTth6CeFziPAAEUMEjJFHWJg1dDex5QxUXKNqRBE
9ek1PMvLhpbwZe7kTMyCVY5VNqrdSPPoruFPQtbxnZyf
```

Undelegated due to the use of an outdated binary:
```
9UHXFYuMLhuugndt8xCFRydmDPFyEEUHYc72tNANEtHp
5Y86A7fUX3LYVDDeoujtAiZFudYcHJq6gw8nsp71wN7U
HYWjn6yL8y7TBPFL9bTgDm6tHgyoEQupgJuBhLLoA5EY
4JCpbdhiQFKWwhrbkNDbwcwBGZnvU4WQrF2vqQLfmZvW
2f7JaYmmrMQQMczLX32ogfP7PBHeyPKbAVNjjEsExZVd
9TW55JrsFhsMoe3Tf8LBR4bPSCX86VXyvioMmCw9tWB
AyN34XqUi5XxgjmivWG2z6TftkqAFjVV5C9zCbx8Fvp5
skNS4zNsKdbbUR9wFTJoPdmReW4NdrDEpp8512TNG4f
DztUnMKM545sdipgqhCsPNhK3YVmBbS2fp9HZgM5Jpw9
GnLmx1s7g9nH3uLRhGpaXTbQEhCSKB6YenBQWQhthSx9
GoJjAkH5hpcPYeW7JDUVfHdqgcufjwdhY2PLwBGJV3Ar
EdHVMTXpLiBbvCUnEoSPQ86pBNY1h9HtL34Q7cpNPWCy
```

Undelegated due to increased operation costs or profit margin:
```
Erw9AQ4UJCgCiAWisUWbFk9Yedm8qvW4YQqmJRrBrE5p
BVDVtmNbZRgPKU81uBkrgfj5TnhtZqQcPAwxD48jcfMd
36nmH3kawhAsNA6sxFva2HgTnQHQDbcrRefvWWbmhHvY
2831fyXRAJ88x1Pd5aW7utw7WH1XkHZEfoWhLk2foLxJ
AMDS4cib433iRstwP9mWnZ4zPqb6hm6uPF7PpvhSkpYC
DE9eEeVsuiKeVfwebg5HYsebqRUvxd7LWsT9hQUtVrTQ
FAKhiQ8nW5sAWAxks1WB8u1MAWsapToCSE3KmF9LuGRQ
```

Undelegated due to being blacklisted for extensive period
```
sjL9n9ymxfWWwkQJxXdsMkdwamXfh3AJ3vCe7rJ8RrT
E2HAJrHnk56QZDUCkcjc4i4pVEqtyuPYL5bNFYtweQuL
4PytR3tmodsvqGTKdY47yie8kmrkARQdb5Ht3Ro3ChH4
```
~~~

---

## `v2024.11-wedel`

- [Release binaries](https://github.com/nymtech/nym/releases/tag/nym-binaries-v2024.11-wedel)
- [Release CHANGELOG.md](https://github.com/nymtech/nym/blob/nym-binaries-v2024.11-wedel/CHANGELOG.md)
- [`nym-node`](nodes/nym-node.md) version `1.1.8`

```sh
Binary Name:        nym-node
Build Timestamp:    2024-09-27T11:02:37.073944654Z
Build Version:      1.1.8
Commit SHA:         c3ec970a377adb25d57be5428551fada2ec55128
Commit Date:        2024-09-26T08:24:53.000000000+02:00
Commit Branch:      master
rustc Version:      1.80.1
rustc Channel:      stable
cargo Profile:      release
```

### Features

- [New Network Monitor](https://github.com/nymtech/nym/pull/4610): Monitors the Nym network by sending itself packages across the mixnet. Network monitor is running two tokio tasks, one manages mixnet clients and another manages monitoring itself. Monitor is designed to be driven externally, via an `HTTP api`. This means that it does not do any monitoring unless driven by something like [`locust`](https://locust.io/). This allows us to tailor the load externally, potentially distributing it across multiple monitors. Includes a dockerised setup for automatically spinning up monitor and driving it with locust.
    - *Note: NNM is not deployed on mainnet yet!*
 
- [Add get_mixnodes_described to validator_client](https://github.com/nymtech/nym/pull/4725)

- [Remove deprecated mark_as_success and use new disarm](https://github.com/nymtech/nym/pull/4751): Update function name to keep terminology consistent with tokio `CancellationToken DropGuard`.

- [Update peer refresh value](https://github.com/nymtech/nym/pull/4754): `lso` expose the value by moving it to wireguard types, and separate the refresh time to the database sync time, so that more probable and needed actions happen faster (refresh) and more improbable ones don't overload the system (peer suspended or stale)
~~~admonish example collapsible=true title='Testing steps performed'
- **Noted** that the constants `DEFAULT_PEER_TIMEOUT` and `DEFAULT_PEER_TIMEOUT_CHECK` have been moved to `common/wireguard-types/src/lib.rs` and are now being used across modules for consistency
- **Observed** that the `peer_controller.rs` now separates the in-memory updates from the storage sync operations to reduce system load
- **Identified** that in-memory updates of peer bandwidth usage happen every `DEFAULT_PEER_TIMEOUT_CHECK` (every 5 seconds), while storage updates occur every 5 * `DEFAULT_PEER_TIMEOUT_CHECK` (every 25 seconds)
 
**Checked System Load and Performance:**
 
- **Monitored** system resource usage (CPU, memory, I/O) during the test to assess the impact of the changes
- **Confirmed** that the separation of in-memory updates and storage syncs resulted in reduced system load, particularly I/O operations, compared to previous versions where storage updates occurred more frequently
- **Ensured** that the system remained responsive and no performance bottlenecks were introduced

- **Efficiency Improvement:** The separation of in-memory updates and storage syncs effectively reduced unnecessary database writes, improving system efficiency without compromising data accuracy
~~~

- [Remove duplicate stat count for retransmissions](https://github.com/nymtech/nym/pull/4756)

- [Make gateway latency check generic](https://github.com/nymtech/nym/pull/4759): Replace concrete gateway type with trait in latency check, so we can make use of it in the vpn client.
~~~admonish example collapsible=true title='Testing steps performed'
- Initialised new `nym-client` with the `--latency-based-selection` flag and ensured it still works as normal. 
~~~

- [chore: remove repetitive words](https://github.com/nymtech/nym/pull/4763)

- [Avoid race on ip and registration structures](https://github.com/nymtech/nym/pull/4766): To avoid a state where the ip is being cleared out before the registration is also cleared out, couple the two structures under the same lock, since they are anyway very inter-dependent.
~~~admonish example collapsible=true title='Testing steps performed'
1.  - **Checked out** the release/2024.10-wedel branch containing the fix for the race condition on IP and registration structures
    - **Deployed** the on a controlled test environment to prevent interference
 
2. **Monitored Logs:**

    - **Enabled** debug logging to capture all events
    - **Monitored** logs in real-time to observe the handling of concurrent registration requests
    - **Checked** for any error messages, warnings, or indications of race conditions
 
3. **Verified Client Responses:**
 
    - Ensured that all clients received appropriate responses:
    - Successful registration with assigned IP and registration data
    - Appropriate error messages if no IPs were available or if other issues occurred
    - Confirmed that no clients were left in an inconsistent state (e.g., assigned an IP but not fully registered)
 
4. **Validated Normal Operation:**
    - **Conducted standard registration processes** with individual clients to confirm that regular functionality is unaffected via `nym-vpn-cli`
    - Ensured that authenticated clients could communicate over the network as expected
~~~

- [Persist used wireguard private IPs](https://github.com/nymtech/nym/pull/4771)

- [Enable dependabot version upgrades for root rust workspace](https://github.com/nymtech/nym/pull/4778)

- [Fix clippy for `unwrap_or_default`](https://github.com/nymtech/nym/pull/4783): Fix nightly build for [beta toolchain](https://github.com/nymtech/nym/actions/runs/10552082396/job/29230401668)

- [Update dependabot](https://github.com/nymtech/nym/pull/4796): Bump max number of dependabot rust PRs to 10. Add readme entry to workspace package. 

- [Run `cargo-autoinherit` for a few new crates](https://github.com/nymtech/nym/pull/4801): Run cargo-autoinherit for a few new crates - Sort crates list. 

- [Add `axum` server to `nym-api`](https://github.com/nymtech/nym/pull/4803): Summary PR to add axum functionality behind a feature flag `axum`, alongside rocket. 

- [Remove unused wireguard flag from SDK](https://github.com/nymtech/nym/pull/4823)

- [Expose wireguard details on self described endpoint](https://github.com/nymtech/nym/pull/4825) 
~~~admonish example collapsible=true title='Testing steps performed'
Wireguard details are now visible at the nym-node endpoint `/api/v1/gateway/client-interfaces` as well as on the nym-api self-described endpoint `/api/v1/gateways/described`, above the existing data displaying mixnet_websocket information. 
 
An example of what will be shown is: 
```json
 "wireguard": {
 "port": 51822,
 "public_key": "<some public key here>"
 }
```
~~~

- [Revamped ticketbook serialisation and exposed additional cli methods](https://github.com/nymtech/nym/pull/4827): `wip` branch that includes changes needed for `vpn-api` alongside additional `ecash utils`
~~~admonish example collapsible=true title='Testing steps performed'
Checked the following commands: 
```sh
show-ticket-books # which displays the information about all ticketbooks associated to the client 
import-ticket-book # which imports a normal ticketbook to the client alongside `--full` flag
```

On the cli, the following were added: `import-coin-index-signatures`, `import-expiration-date-signatures` and `import-master-verification-key`.
~~~

- [Run cargo autoinherit following last weeks dependabot updates](https://github.com/nymtech/nym/pull/4831)

- [Remove serde_crate named import](https://github.com/nymtech/nym/pull/4832)

- [Create nym-repo-setup debian package and nym-vpn meta package](https://github.com/nymtech/nym/pull/4837): Create nym-repo-setup debian package that sets up the nymtech debian repo on the system it's installed on. It does 2 things:
 
    1. Copy the keyring to `/usr/share/keyrings/nymtech.gpg`
    2. Copy the repo spec to `/etc/apt/sources.list.d/nymtech.list`
    - Also create a meta package `nym-vpn` which only purpose is to depend on the daemon and UI.

~~~admonish example collapsible=true title='Usage'
1. Install with
```sh
sudo dpkg -i ./nym-repo-setup.deb
```
2. Once it's installed, it should be possible to install the vpn client with
```sh
sudo apt install nym-vpnc
```
3. To reemove the repo, use
```sh
sudo apt remove nym-repo-setup
```

NOTE: removing the repo will not remove any installed nym-vpn packages
~~~

~~~admonish example collapsible=true title='Testing steps performed'

1. **Downloaded** the `nym-repo-setup.deb` package to a Debian-based test system
 
2. **Installed** the repository setup package using the command: 
```bash
sudo dpkg -i ./nym-repo-setup.deb
```
 
3. **Verified** that the GPG keyring was copied to `/usr/share/keyrings/nymtech.gpg`:
```bash
ls -l /usr/share/keyrings/nymtech.gpg
```
 
4. **Checked** that the repository specification was added to `/etc/apt/sources.list.d/nymtech.list`:
```bash
cat /etc/apt/sources.list.d/nymtech.list
```
 
 5. **Updated** the package list:
```bash
sudo apt update
```
 
6. **Installed** the VPN client meta-package:
```bash
sudo apt install nym-vpnc
```

7. **Confirmed** that the `nym-vpnc` package and its dependencies (daemon and UI) were installed successfully

8. **Tested** the VPN client to ensure it operates as expected
 
9. **Removed** the repository setup package:
```bash
sudo apt remove nym-repo-setup
```

10. **Verified** that the repository specification file `/etc/apt/sources.list.d/nymtech.list` was removed
 
11. **Ensured** that the installed `nym-vpnc` packages remained installed and functional after removing the repo setup package
~~~

- [Use ecash credential type for bandwidth value](https://github.com/nymtech/nym/pull/4840)

- [Start switching over jobs to arc-ubuntu-20.04](https://github.com/nymtech/nym/pull/4843)

~~~admonish example collapsible=true title='`ci-binary-config-checker`'
```
 - ci-build-upload-binaries
 - ci-build
 - ci-cargo-deny
 - ci-contracts-schema
 - ci-contracts-upload-binaries
 - ci-contracts
 - ci-docs
 - ci-nym-wallet-rust
 - ci-sdk-wasm
```
~~~

- [Move credential verification into common crate](https://github.com/nymtech/nym/pull/4853)

- [Revert runner for `ci-docs`](https://github.com/nymtech/nym/pull/4855)

- [Remove `golang` workaround in `ci-sdk-wasm`](https://github.com/nymtech/nym/pull/4858)

- [Fix linux conditional in `ci-build.yml`](https://github.com/nymtech/nym/pull/4863)

- [Disable push trigger and add missing paths in `ci-build`](https://github.com/nymtech/nym/pull/4864)

- [chore: removed completed queued mixnet migration](https://github.com/nymtech/nym/pull/4865)

- [Bump defguard to github latest version](https://github.com/nymtech/nym/pull/4872)

- [Backport #4894 to fix ci](https://github.com/nymtech/nym/pull/4899)

### Bugfix

- [Fix test failure in ipr request size](https://github.com/nymtech/nym/pull/4844): Nightly build started failing due to a unit test using `now()`, changing the serialized size. Fixed to use a fixed date.

- [Fix clippy for nym-wallet and latest rustc](https://github.com/nymtech/nym/pull/4845)

- [Allow updating globally stored signatures](https://github.com/nymtech/nym/pull/4891)

- [Bugfix/ticketbook false double spending](https://github.com/nymtech/nym/pull/4892)
~~~admonish example collapsible=true title='Testing steps performed'
Tested running a client in mixnet mode, with a standard ticketbook, as well as a client using an imported ticketbook. The double spending bug is no longer an issue, bandwidth is consumed properly, and upon consumption of one ticket another ticket is properly obtained. 
~~~

### Operators Guide, Tooling & Updates

- [WSS setup guide updates](https://github.com/nymtech/nym/commit/05d6652177fb77324f8c38b3d8a547d07e729fec): Operators setting up WSS and reverse proxy on Gateways have now cleaner and simpler guide to configure their VPS. 

- [Updat hostname instruction for WSS](https://github.com/nymtech/nym/commit/7146c4c012ba7012dc74edc8510bbf377dc32fba): Adding a hostname instruction for clarity

## `nym-node` patch from `release/2024.10-caramello`

- [Patch release binaries](https://github.com/nymtech/nym/releases/tag/nym-binaries-v2024.10-caramello-patch)

```sh
Binary Name:        nym-node
Build Timestamp:    2024-09-16T15:00:41.019107021Z
Build Version:      1.1.7
Commit SHA:         65c8982cab0ff3a1154966e7d61956cb42a065fc
Commit Date:        2024-09-16T15:59:34.000000000+02:00
Commit Branch:      HEAD
rustc Version:      1.81.0
rustc Channel:      stable
cargo Profile:      release
```

This patch fixes [`v202410-caramello`](#v202410-caramello) release [bug](#known-bugs--undone-features) where one of the used dependencies - [`DefGuard`](https://github.com/DefGuard/defguard/issues/619), was failing.

Updating to this patched version and running `nym-node --mode exit-gateway` with `--wireguard-enabled true` should result in a smooth node start without the `defguard_wireguard` error, occuring to some operators before:
```sh
/home/ubuntu/.cargo/registry/src/index.crates.io-6f17d22bba15001f/defguard_wireguard_rs-0.4.2/src/netlink.rs:155: Serialized netlink packet (23240 bytes) larger than maximum size 12288: NetlinkMessage.
```

This release is a patch only, there are no additional features, everything else stays the same like in the latest release [`v202410-caramello`](#v202410-caramello).

## `v2024.10-caramello`

- [Release binaries](https://github.com/nymtech/nym/releases/tag/nym-binaries-v2024.10-caramello)
- [Release CHANGELOG.md](https://github.com/nymtech/nym/blob/nym-binaries-v2024.10-caramello/CHANGELOG.md)
- [`nym-node`](nodes/nym-node.md) version `1.1.7`

~~~admonish example collapsible=true title='CHANGELOG.md'
- Backport 4844 and 4845 ([#4857])
- Bugfix/client registration vol2 ([#4856])
- Remove wireguard feature flag and pass runtime enabled flag ([#4839])
- Eliminate cancel unsafe sig awaiting ([#4834])
- added explicit updateable admin to the mixnet contract ([#4822])
- using legacy signing payload in CLI and verifying both variants in contract ([#4821])
- adding ecash contract address ([#4819])
- Check profit margin of node before defaulting to hardcoded value  ([#4802])
- Sync last_seen_bandwidth immediately ([#4774])
- Feature/additional ecash nym cli utils ([#4773])
- Better storage error logging ([#4772])
- bugfix: make sure DKG parses data out of events if logs are empty ([#4764])
- Fix clippy on rustc beta toolchain ([#4746])
- Fix clippy for beta toolchain ([#4742])
- Disable testnet-manager on non-unix ([#4741])
- Don't set NYM_VPN_API to default ([#4740])
- Update publish-nym-binaries.yml ([#4739])
- Update ci-build-upload-binaries.yml ([#4738])
- Add NYM_VPN_API to network config ([#4736])
- Re-export RecipientFormattingError in nym sdk ([#4735])
- Persist wireguard peers ([#4732])
- Fix tokio error in 1.39 ([#4730])
- Feature/vesting purge plus ranged cost params ([#4716])
- Fix (some) feature unification build failures ([#4681])
- Feature Compact Ecash : The One PR ([#4623])

[#4857]: https://github.com/nymtech/nym/pull/4857
[#4856]: https://github.com/nymtech/nym/pull/4856
[#4839]: https://github.com/nymtech/nym/pull/4839
[#4834]: https://github.com/nymtech/nym/pull/4834
[#4822]: https://github.com/nymtech/nym/pull/4822
[#4821]: https://github.com/nymtech/nym/pull/4821
[#4819]: https://github.com/nymtech/nym/pull/4819
[#4802]: https://github.com/nymtech/nym/pull/4802
[#4774]: https://github.com/nymtech/nym/pull/4774
[#4773]: https://github.com/nymtech/nym/pull/4773
[#4772]: https://github.com/nymtech/nym/pull/4772
[#4764]: https://github.com/nymtech/nym/pull/4764
[#4746]: https://github.com/nymtech/nym/pull/4746
[#4742]: https://github.com/nymtech/nym/pull/4742
[#4741]: https://github.com/nymtech/nym/pull/4741
[#4740]: https://github.com/nymtech/nym/pull/4740
[#4739]: https://github.com/nymtech/nym/pull/4739
[#4738]: https://github.com/nymtech/nym/pull/4738
[#4736]: https://github.com/nymtech/nym/pull/4736
[#4735]: https://github.com/nymtech/nym/pull/4735
[#4732]: https://github.com/nymtech/nym/pull/4732
[#4730]: https://github.com/nymtech/nym/pull/4730
[#4716]: https://github.com/nymtech/nym/pull/4716
[#4681]: https://github.com/nymtech/nym/pull/4681
[#4623]: https://github.com/nymtech/nym/pull/4623
~~~

### Features

- [Add 1GB/day/user bandwidth cap](https://github.com/nymtech/nym/pull/4717)

~~~admonish example collapsible=true title='Testing steps performed'
**Scenario 1: Bandwidth Decreasing Continuously**

1. Started the client and noted the initial bandwidth (e.g., 1GB).
2. Used the client and tracked bandwidth usage over time (e.g., decrease by 100MB every hour).
3. Restarted the client after some usage.
4. Verified the bandwidth continued from the last recorded value, not reset.

The bandwidth continued decreasing without resetting upon restart. Logs and reports correctly reflected the decreasing bandwidth.

**Scenario 2: Bandwidth Reset Next Day**

1. Used the client normally until the end of the day.
2. Suspended some clients and kept others active.
3. Checked bandwidth at midnight.
4. Verified that bandwidth reset to 1GB for both suspended and active clients.

Bandwidth reset to 1GB for all clients at midnight. Logs and reports correctly showed the reset.

**Scenario 3: Bandwidth Reset at a Different Time (e.g., Midday)**

1. Configured the system to reset bandwidth at midday.
2. Used the client and monitored bandwidth until midday.
3. Kept the client connected during the reset time.
4. Verified that bandwidth reset to 1GB live at midday.

Bandwidth reset to 1GB at midday while the client was connected. Logs and reports correctly reflected the reset.

**Scenario 4: Stale Check for 3 Days**

1. Kept a client inactive for 3 days.
2. Verified removal from the peer list after 3 days.
3. Reconnected the client after 3 days and checked for a new private IP.
4. Restarted a client within 3 days and verified it retained the same private IP.

The client was removed from the peer list after 3 days of inactivity. Upon re-connection after 3 days, the client received a new private IP. The client retained the same private IP when restarted within 3 days.
~~~

- [Feature/merge back](https://github.com/nymtech/nym/pull/4710): Merge back from the release branch the changes that fix the `nym-node` upgrades

- [Removed mixnode/gateway config migration code and disabled cli without explicit flag](https://github.com/nymtech/nym/pull/4706): Commands for archived / legacy binaries `nym-gateway` and `nym-mixnode` won't do anything without explicit `--force-run` to bypass the deprecation. The next step, in say a month or so, is to completely remove all `cli` related things.

~~~admonish example collapsible=true title='Testing steps performed'
- Verify that the `nym-gateway` binary and `nym-mixnode` binary commands return the _error message_ stating to update to nym-node
- Check that when adding the `--force-run` flag, it still allows the command to be run (aside from `init` which has been removed) and the message stating to update to nym-node is a _warning_ now
- Check `nym-node` is not affected
- Reviewed the changes in the PR
~~~

- [Handle clients with different versions in IPR](https://github.com/nymtech/nym/pull/4723): Allow the IPR to handle clients connecting both using `v6` and `v7`, independently. The motivation is that we want to be able to roll out a API version change gradually for VPN clients without breaking backwards compatibility. The main feature on the new `v7` format that is not yet used, is that it adds signatures for connect/disconnect.

~~~admonish example collapsible=true title='Testing steps performed'
Run the same command (using same gateways deployed from this PR) on different versions of the `nym-vpn-cli`.

Example:
```sh
~/nym-vpn-core-v0.1.10_macos_universal ❯ sudo -E ./nym-vpn-cli -c ../qa.env run --entry-gateway-id $entry_gateway --exit-gateway-id $exit_gateway --enable-two-hop

~/nym-vpn-core-v0.1.11-dev_macos_universal ❯ sudo -E ./nym-vpn-cli -c ../qa.env run --entry-gateway-id $entry_gateway --exit-gateway-id $exit_gateway --enable-two-hop
```
~~~

- [Backport `#4844` and `#4845`](https://github.com/nymtech/nym/pull/4857)

- [Remove wireguard feature flag and pass runtime enabled flag](https://github.com/nymtech/nym/pull/4839)

- [Eliminate cancel unsafe sig awaiting](https://github.com/nymtech/nym/pull/4834)

- [Added explicit updateable admin to the mixnet contract](https://github.com/nymtech/nym/pull/4822)

- [Using legacy signing payload in CLI and verifying both variants in contract](https://github.com/nymtech/nym/pull/4821)

- [Adding ecash contract address](https://github.com/nymtech/nym/pull/4819)

- [Check profit margin of node before defaulting to hardcoded value ](https://github.com/nymtech/nym/pull/4802)

- [Sync `last_seen_bandwidth` immediately](https://github.com/nymtech/nym/pull/4774)

- [Feature/additional ecash `nym-cli` utils](https://github.com/nymtech/nym/pull/4773)

- [Better storage error logging](https://github.com/nymtech/nym/pull/4772)

- [Disable testnet-manager on non-unix](https://github.com/nymtech/nym/pull/4741)

- [Don't set NYM_VPN_API to default](https://github.com/nymtech/nym/pull/4740)

- [Update publish-nym-binaries.yml](https://github.com/nymtech/nym/pull/4739): Adds wireguard to builds

- [Update ci-build-upload-binaries.yml](https://github.com/nymtech/nym/pull/4738): Adds wireguard for ci-builds

- [Add NYM_VPN_API to network config](https://github.com/nymtech/nym/pull/4736)

- [Re-export RecipientFormattingError in nym sdk](https://github.com/nymtech/nym/pull/4735)

- [Persist wireguard peers](https://github.com/nymtech/nym/pull/4732)

- [Feature/vesting purge plus ranged cost params](https://github.com/nymtech/nym/pull/4716): Combines [\#4715](https://github.com/nymtech/nym/pull/4715) and [\#4711](https://github.com/nymtech/nym/pull/4711) into one.
    - Disables all non-essential operations on the vesting contract => you can no longer bond mixnodes/make delegations/etc. (you can still, however, withdraw your vested tokens and so on)
    - Introduces admin-controlled minimum (and maximum) profit margin and interval operating costs.
    - both contracts have to be migrated **at the same time**. ideally within the same transaction
    - mixnet contract migration is not allowed (and will fail) if there are any pending actions involving vesting tokens, like delegating, increasing pledge, etc

- [Bump braces from `3.0.2` to `3.0.3` in `/nym-wallet/webdriver`](https://github.com/nymtech/nym/pull/4709): Bumps [braces](https://github.com/micromatch/braces) from `3.0.2` to `3.0.3`.

### Bugfix

- [chore: fix 1.80 lint issues](https://github.com/nymtech/nym/pull/4731)

~~~admonish example collapsible=true title='Testing steps performed'
- Building all binaries is ok
- Running `cargo fmt` returns no issues
~~~

- [Fix version 1 not having template correspondent initially](https://github.com/nymtech/nym/pull/4733)

~~~admonish example collapsible=true title='Testing steps performed'
Tested updating an old `nym-node` version and ensuring it did not throw any errors.
~~~

- [Bugfix/client registration vol2](https://github.com/nymtech/nym/pull/4856)

- [Fix tokio error in `1.39`](https://github.com/nymtech/nym/pull/4730):
    - Bump tokio to `1.39.2`, skipping the issue with `1.39.1`


- [Fix (some) feature unification build failures](https://github.com/nymtech/nym/pull/4681): Running a script in the root workspace that builds each crate independently

~~~admonish example collapsible=true title='The script'
 ```sh
 #!/bin/bash

 packages=$(cargo metadata --format-version 1 --no-deps | jq -r '.packages[].name')

 # Loop through each package and build
 for package in $packages; do
     echo "Building $package"
     cargo clean
     cargo check -p "$package"
     if [ $? -ne 0 ]; then
         echo "Build failed for $package. Stopping."
         exit 1
     fi
 done
 ```
~~~

- [bugfix: make sure DKG parses data out of events if logs are empty](https://github.com/nymtech/nym/pull/4764): This will be the case on post `0.50` chains

- [Fix clippy on rustc beta toolchain](https://github.com/nymtech/nym/pull/4746): Fix clippy warnings for rust beta toolchain

- [Fix clippy for beta toolchain](https://github.com/nymtech/nym/pull/4742): Fix beta toolchain clippy by removing unused module
    - Add `nym-` prefix to `serde-common` crate
    - Remove ignored `default-features = false` attribute for workspace dependency

### Crypto

- [Feature Compact Ecash : The One PR](https://github.com/nymtech/nym/pull/4623)

### Operators Guide, Tooling & Updates

- More explicit [setup for `nym-node`](nodes/setup.md#initialise--run) with a new [option explanation](nodes/setup.md#essential-parameters--variables), including syntax examples

- New [VPS networking configuration steps for Wireguard](nodes/configuration.md#routing-configuration)

- Wireguard [builds from source](binaries/building-nym.md) together with `nym-node`, no need to specify with a feature flag anymore

- Wireguard peers stay connected for longer time, re-connections are also faster

- Profit margin and operating cost values are set to the values agreed by operators off-chain vote, the values can be changed in the future through [Nym Operators governance process](https://forum.nymtech.net/t/poll-proposal-for-on-chain-minimum-profit-margin-for-all-nym-nodes/253)
```admonish success title=""
- Minimum profit margin = 20%
- Maximum profit margin = 50%
- Minimum operating cost = 0 NYM
- Maximum operating cost = 1000 NYM
```

- [Nym Harbourmater](https://harbourmaster.nymtech.net) has several new functionalities:
    - Version counting graph for Gateways and Mixnodes
    - Several new columns with larger nodes performance and settings overview.
    - Top routing score now includes:
        - Wireguard registration and complete handshake test, to configure see [tasklist below](#operators-tasks)
        - DNS resolution check, to configure see [tasklist below](#operators-tasks)
        - Wireguard perfomance > 0.75, to configure see [tasklist below](#operators-tasks)

- New [Nym Wallet](https://github.com/nymtech/nym/releases/tag/nym-wallet-v1.2.14) is out!
    - Vesting contract functionalities have been purged, users can only remove tokens from vesting
    - Migrating from `mixnode` or `gateway` smart contracts to a new unifying `nym-node` smart contract will be available soon using Nym desktop wallet, just like you are used to for bonding and node settings. After this migration all `nym-nodes` will be able to receive delegation and rewards. We will share a step by step guide once this migration will be deployed. No action needed now.

- [Nym API Check CLI](testing/node-api-check.md) is upgraded according to the latest API endpoints, output is cleaner and more concise.


#### Operators Tasks

```admonish warning title=""
**The steps below are highly recommended for all operators and mandatory for everyone who is a part of Nym Delegation or Grant program. Deadline is Friday, September 20th, 2024.**
```

Every `nym-node` should be upgraded to the latest version! Operators can test using [Sandbox env](sandbox.md) during the pre-release period, then upgrade on mainnet. During the upgrade, please follow the points below before you restart the node:

**`nym-node`**

- Make sure to fill in basic description info, into the file located at `.nym/nym-nodes/<ID>/data/description.toml` (all nodes)
- Configure wireguard routing with new [`network_tunnel_manager.sh`](https://gist.github.com/tommyv1987/ccf6ca00ffb3d7e13192edda61bb2a77) following [these steps](nodes/configuration.md#routing-configuration) (Gateways only for the time being)
- Enable Wireguard with `--wireguard-enabled true` flag included in your run command (Gateways only for the time being)
    - Note: On some VPS this setup may not be enough to get the correct results as some ISPs  have their own security groups setup below the individual VPS. In that case a ticket to ISP will have to be issued to open the needed settings. We are working on a template for such ticket.
- Setup [reverse proxy and WSS](nodes/proxy-configuration.md) on `nym-node` (Gateways only for the time being)
- Don't forget to restart your node - or (preferably using [systemd automation](nodes/configuration.md#systemd)) reload daemon and restart the service
- Optional: Use [`nym-gateway-probe`](testing/gateway-probe.html) and [NymVPN CLI](https://nymtech.net/developers/nymvpn/cli.html) to test your own Gateway
- Optional: Run the script below to measure ping speed of your Gateway and share your results in [Nym Operators channel](https://matrix.to/#/#operators:nymtech.chat)

~~~admonish example collapsible=true title='The script to measure Gateway ping results'
We made a script for pinging nymtech.net from your GWs. Can you please install it and then share the result together with your Gateway ID:

1. Get the script onto your machine (soon on github for curl or wget):

```sh
# paste all this block as one command
cat <<'EOL' > ping_with_curl_average_for_wg_check.sh
#!/bin/bash

ping_with_curl_average_for_wg_check() {
    total_connect_time=0
    total_total_time=0
    iterations=5
    timeout=2

    for ((i=1; i<=iterations; i++)); do
        echo "ping attempt $i..."

        echo "curling nymtech.net to check ping response times"
        times=$(curl -I https://nymtech.net --max-time $timeout \
        -w "time_connect=%{time_connect}\ntime_total=%{time_total}" -o /dev/null -s)

        time_connect=$(echo "$times" | grep "time_connect" | cut -d"=" -f2)
        time_total=$(echo "$times" | grep "time_total" | cut -d"=" -f2)

        total_connect_time=$(echo "$total_connect_time + $time_connect" | bc)
        total_total_time=$(echo "$total_total_time + $time_total" | bc)

        echo "time to connect: $time_connect s"
        echo "total time: $time_total s"
    done

    average_connect_time=$(echo "scale=3; $total_connect_time / $iterations" | bc)
    average_total_time=$(echo "scale=3; $total_total_time / $iterations" | bc)

    echo "-----------------------------------"
    echo "average time to connect: $average_connect_time s"
    echo "average total time: $average_total_time s"
}

ping_with_curl_average_for_wg_check
EOL
```

2. Make executable:

```sh
chmod +x ping_with_curl_average_for_wg_check.sh
```

3. In case you don't have `bc`, install it:

```sh
sudo apt install bc
```

4. Run:

```sh
./ping_with_curl_average_for_wg_check.sh
```

5. Share results and ID key in [Nym Operators channel](https://matrix.to/#/#operators:nymtech.chat)

THANK YOU!
~~~

**validators**

- Validators need to update and prepare for ecash implementation.

### Known Bugs & Undone features

- New `nym-nodes` without a performance 24h history above 50% don't show routing properly on `nym-gateway-probe`, on Nym Harbourmaster the page may appear blank - we are working on a fix.
- Wireguard works on IPv4 only for the time being, we are working on IPv6 implementation.
- Harbourmaster *Role* column shows `nym-node --mode exit-gateway` as `EntryGateway`, we are working to fix it.
- In rare occassions Harbourmaster shows only *"panda"* without the *"smiley"* badge even for nodes, which have T&C's accepted. We are working to fix it.
- Sometimes `nym-node` running with `--wireguard-enabled true` gives this error on restart: `Serialized netlink packet .. larger than maximum size ..`
```sh
/home/ubuntu/.cargo/registry/src/index.crates.io-6f17d22bba15001f/defguard_wireguard_rs-0.4.2/src/netlink.rs:155: Serialized netlink packet (23240 bytes) larger than maximum size 12288: NetlinkMessage.
```

From what we found out it seems that one of our [dependencies - `DefGuard` - is failing](https://github.com/DefGuard/defguard/issues/619). Based on the reading on their fix, it seems that when node operators try to re-create a wireguard interface with too many previous peers (like on Gateway restart, with restoring from storage), there's an overflow. So their fix is to just add them one by one. To be sure that bumping the dependency version fixes the problem there's still two things we'd need to check - and your feedback would help us a lot:

1. Did operators only encounter this error after a `nym-node` (Gateway) restart?
2. Reprouce this error ourselves and see if it actually fixes our problem.

**Please share your experience with us to help faster fix of this issue.**

---

## `v2024.9-topdeck`

- [Release binaries](https://github.com/nymtech/nym/releases/tag/nym-binaries-v2024.9-topdeck)
- [Release CHANGELOG.md](https://github.com/nymtech/nym/blob/nym-binaries-v2024.9-topdeck/CHANGELOG.md)
- [`nym-node`](nodes/nym-node.md) version `1.1.6`

~~~admonish example collapsible=true title='CHANGELOG.md'
- chore: fix 1.80 lint issues ([#4731])
- Handle clients with different versions in IPR ([#4723])
- Add 1GB/day/user bandwidth cap ([#4717])
- Feature/merge back ([#4710])
- removed mixnode/gateway config migration code and disabled cli without explicit flag ([#4706])

[#4731]: https://github.com/nymtech/nym/pull/4731
[#4723]: https://github.com/nymtech/nym/pull/4723
[#4717]: https://github.com/nymtech/nym/pull/4717
[#4710]: https://github.com/nymtech/nym/pull/4710
[#4706]: https://github.com/nymtech/nym/pull/4706
~~~

### Features

* [Removed `nym-mixnode` and `nym-gateway` config migration code and disabled CLI without explicit flag](https://github.com/nymtech/nym/pull/4706): Gateway and Mixnode commands now won't do anything without explicit `--force-run` to bypass the deprecation, instead it will tell an operator to run a `nym-node`.  The next step, in say a month or so, is to completely remove all `cli` related things.
~~~admonish example collapsible=true title='Testing steps performed'
- Verify that the `nym-gateway` binary and `nym-mixnode` binary commands return the `_error message_` stating to *update to `nym-node`*
- Check that when adding the `--force-run` flag, it still allows the command to be run (aside from `init` which has been removed) and the message stating to update to `nym-node` is a `_warning_` now
- Check `nym-node` is not affected
- Review the changes in the PR
~~~

* [Add 1GB/day/user bandwidth cap](https://github.com/nymtech/nym/pull/4717)

~~~admonish example collapsible=true title='Testing steps performed - Scenario 1: Bandwidth Decreasing Continuously'
1. Start the client and noted the initial bandwidth (e.g., 1GB).
2. Us the client and track bandwidth usage over time (e.g., decrease by 100MB every hour).
3. Restart the client after some usage.
4. Verify the bandwidth continued from the last recorded value, not reset.

**Notes:**
 The bandwidth continued decreasing without resetting upon restart. Logs and reports correctly reflected the decreasing bandwidth.
~~~

~~~admonish example collapsible=true title='Testing steps performed - Scenario 2: Bandwidth Reset Next Day'
1. Use the client normally until the end of the day.
2. Suspend some clients and kept others active.
3. Check bandwidth at midnight.
4. Verify that bandwidth reset to 1GB for both suspended and active clients.

**Notes:**
Bandwidth reset to 1GB for all clients at midnight. Logs and reports correctly showed the reset.
~~~

~~~admonish example collapsible=true title='Testing steps performed - Scenario 3: Bandwidth Reset at a Different Time (e.g., Midday)'
1. Configure the system to reset bandwidth at midday.
2. Use the client and monitored bandwidth until midday.
3. Keep the client connected during the reset time.
4. Verify that bandwidth reset to 1GB live at midday.

**Notes:**
Bandwidth reset to 1GB at midday while the client was connected. Logs and reports correctly reflected the reset.
~~~

* [Handle clients with different versions in IPR](https://github.com/nymtech/nym/pull/4723): Allow the IPR to handle clients connecting both using `v6` and `v7`, independently. The motivation is that we want to be able to roll out an API version change gradually for NymVPN clients without breaking backwards compatibility. The main feature on the new `v7` format that is not yet used, is that it adds signatures for connect/disconnect.
~~~admonish example collapsible=true title='Testing steps performed'
Run the same command (using same gateways deployed from this PR) on different versions of the `nym-vpn-cli`.

Example:
```sh
sudo -E ./nym-vpn-cli -c ../qa.env run --entry-gateway-id $entry_gateway --exit-gateway-id $exit_gateway --enable-two-hop

sudo -E ./nym-vpn-cli -c ../qa.env run --entry-gateway-id $entry_gateway --exit-gateway-id $exit_gateway --enable-two-hop
```
~~~

### Bugfix

* [Feature/merge back](https://github.com/nymtech/nym/pull/4710): Merge back from the release branch the changes that fix the `nym-node` upgrades.

* [Fix version `1.x.x` not having template correspondent initially](https://github.com/nymtech/nym/pull/4733): This should fix the problem of config deserialisation when operators upgrade nodes and skip over multiple versions.
~~~admonish example collapsible=true title='Testing steps performed'
- Tested updating an old nym-node version and ensuring it did not throw any errors.
~~~

* [chore: fix 1.80 lint issues](https://github.com/nymtech/nym/pull/4731):
~~~admonish example collapsible=true title='Testing steps performed'
- Building all binaries is ok
- Running `cargo fmt` returns no issues
~~~

### Operators Guide updates

* [WireGuard tunnel configuration guide](nodes/configuration.md#routing-configuration) for `nym-node` (currently Gateways functionalities). For simplicity we made a detailed step by step guide to upgrade an existing `nym-node` to the latest version and configure your VPS routing for WireGuard. Open by clicking on the example block below.

~~~admonish example collapsible=true title='Upgrading  `nym-node` with WG'
**Prerequisites**

- **Nym Node Version:** You must be running the `2024.9-topdeck` release branch, which operates as `nym-node` version `1.1.6`. You can find the release here: [Nym 2024.9-topdeck Release](https://github.com/nymtech/nym/releases/tag/nym-binaries-v2024.9-topdeck).

- **Important:** Before proceeding, make sure to [back up](nodes/maintenance.md#backup-a-node) your current `nym-node` configuration to avoid any potential data loss or issues.


- **Download Nym Node:**
    - You can download the `nym-node` binary directly using the following command:
```bash
curl -L https://github.com/nymtech/nym/releases/download/nym-binaries-v2024.9-topdeck/nym-node -o nym-node && chmod u+x nym-node
```

**Step 1: Update UFW Firewall Rules**

- **Warning:** Enabling the firewall with UFW without allowing SSH port 22 first will lead to losing access over SSH. Make sure port 22 is allowed before proceeding with any UFW configurations.

Run the following as root or with `sudo` prefix:

1. Check the current status of UFW (Uncomplicated Firewall):
```bash
ufw status
```

2. Ensure that the following ports are allowed on your machine before adding the WireGuard port:

```bash
ufw allow 22/tcp    # SSH - you're in control of these ports
ufw allow 80/tcp    # HTTP
ufw allow 443/tcp   # HTTPS
ufw allow 1789/tcp  # Nym specific
ufw allow 1790/tcp  # Nym specific
ufw allow 8080/tcp  # Nym specific - nym-node-api
ufw allow 9000/tcp  # Nym Specific - clients port
ufw allow 9001/tcp  # Nym specific - wss port
ufw allow 51822/udp # WireGuard
```

3. Confirm that the UFW rules have been updated:
```bash
ufw status
```

**Step 2: Download and Prepare the Network Tunnel Manager Script**

1. Download the [`network_tunnel_manager.sh`](https://gist.github.com/tommyv1987/ccf6ca00ffb3d7e13192edda61bb2a77) script:
```bash
curl -L -o network_tunnel_manager.sh https://gist.githubusercontent.com/tommyv1987/ccf6ca00ffb3d7e13192edda61bb2a77/raw/3c0a38c1416f8fdf22906c013299dd08d1497183/network_tunnel_manager.sh
```

2. Make the script executable:
```bash
chmod u+x network_tunnel_manager.sh
```

3. Apply the WireGuard IPTables rules:
```bash
./network_tunnel_manager.sh apply_iptables_rules_wg
```

**Step 3: Update the Nym Node Service File**

1. Modify your [`nym-node` service file](nodes/configuration.md#systemd) to enable WireGuard. Open the file (usually located at `/etc/systemd/system/nym-node.service`) and update the `[Service]` section as follows:

```ini
[Service]
User=<YOUR_USER_NAME>
Type=simple
#Environment=RUST_LOG=debug
# CAHNGE PATH IF YOU DON'T RUN IT FROM ROOT HOME DIRECTORY
ExecStart=/root/nym-node run --mode exit-gateway --id <YOUR_NODE_LOCAL_ID> --accept-operator-terms-and-conditions --wireguard-enabled true
Restart=on-failure
RestartSec=30
StartLimitInterval=350
StartLimitBurst=10
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target

# ADD OR TWEAK ANY CUSTOM SETTINGS
```

2. Reload the systemd daemon to apply the changes:
```bash
systemctl daemon-reload
```

3. Restart the `nym-node service`:
```bash
systemctl restart nym-node.service
```

4. Optionally, you can check if the node is running correctly by monitoring the service logs:
```bash
journalctl -u nym-node.service -f -n 100
```

**Step 4: Run the Network Tunnel Manager Script**

Finally, run the following command to initiate our favorite routing test - run the joke through the WireGuard tunnel:
```bash
./network_tunnel_manager.sh joke_through_wg_tunnel
```

- **Note:** Wireguard will return only IPv4 joke, not IPv6. WG IPv6 is under development. Running IPR joke through the mixnet with `./network_tunnel_manager.sh joke_through_the_mixnet` should work with both IPv4 and IPv6!
~~~

* [Change `--wireguard-enabled` flag to `true`](nodes/setup.md#-initialise--run): With a proper [routing configuration](nodes/configuration.md#routing-configuration) `nym-nodes` running as Gateways can now enable WG. See the example below:

~~~admonish example collapsible=true title='Syntax to run `nym-node` with WG enabled'
For Exit Gateway:
```sh
./nym-node run --id <ID> --mode exit-gateway --public-ips "$(curl -4 https://ifconfig.me)" --hostname "<YOUR_DOMAIN>" --http-bind-address 0.0.0.0:8080 --mixnet-bind-address 0.0.0.0:1789 --location <COUNTRY_FULL_NAME> --accept-operator-terms-and-conditions --wireguard-enabled true

# <YOUR_DOMAIN> is in format without 'https://' prefix
# <COUNTRY_FULL_NAME> is format like 'Jamaica',  or two-letter alpha2 (e.g. 'JM'), three-letter alpha3 (e.g. 'JAM') or three-digit numeric-3 (e.g. '388') can be provided.
# wireguard can be enabled from version 1.1.6 onwards
```

For Entry Gateway:
```sh
./nym-node run --id <ID> --mode entry-gateway --public-ips "$(curl -4 https://ifconfig.me)" --hostname "<YOUR_DOMAIN>" --http-bind-address 0.0.0.0:8080 --mixnet-bind-address 0.0.0.0:1789 --accept-operator-terms-and-conditions --wireguard-enabled true

# <YOUR_DOMAIN> is in format without 'https://' prefix
# <COUNTRY_FULL_NAME> is format like 'Jamaica',  or two-letter alpha2 (e.g. 'JM'), three-letter alpha3 (e.g. 'JAM') or three-digit numeric-3 (e.g. '388') can be provided.
# wireguard can be enabled from version 1.1.6 onwards
```
~~~

* [Update Nym exit policy](https://nymtech.net/.wellknown/network-requester/exit-policy.txt): Based on the survey, AMA and following discussions we added several ports to Nym exit policy. The ports voted upon in the [forum governance](https://forum.nymtech.net/t/poll-a-new-nym-exit-policy-for-exit-gateways-and-the-nym-mixnet-is-inbound/464) have not been added yet due to the concerns raised. These ports were unrestricted:

~~~admonish example collapsible=true title='Newly opened ports in Nym exit policy'
```
22 # SSH
123 # NTP
445 # SMB file share Windows
465 # URD for SSM
587 # SMTP
853 # DNS over TLS
1433 # databases
1521 # databases
2049 # NFS
3074 # Xbox Live
3306 # databases
5000-5005 # RTP / VoIP
5432 # databases
6543 # databases
8080 # HTTP Proxies
8767 # TeamSpeak
8883 # Secure MQ Telemetry Transport - MQTT over SSL
9053 # Tari
9339 # gaming
9443 # alternative HTTPS
9735 # Lightning
25565 # Minecraft
27000-27050 # Steam and game servers
60000-61000 # MOSH
```
~~~

* [Create a NymConnect archive page](https://nymtech.net/developers/archive/nym-connect.html), PR [\#4750](https://github.com/nymtech/nym/commit/5096c1e60e203dcf8be934823946e24fda16a9a3): Archive deprecated NymConnect for backward compatibility, show PEApps examples for both NC and maintained `nym-socks5-client`.

* Fix broken URLs and correct redirection. PRs: [\#4745](https://github.com/nymtech/nym/commit/7e36595d8fa7706876880b42df1c998a4b8c1478), [\#4752](https://github.com/nymtech/nym/commit/1db61f800c6884e284c5ab21e7abce3bc6d91d99) [\#4755](https://github.com/nymtech/nym/commit/aaf3dca5b999ad7f19d2ff170078b43c9c4476c2), [\#4737](https://github.com/nymtech/nym/commit/6f669866e92e637772726ad05caa5c5501a830f3)
~~~admonish example collapsible=true title='Testing steps performed'
- Use [deadlinkchecker.com](https://www.deadlinkchecker.com/website-dead-link-checker.asp) to go over `nymtech.net` and correct all docs URLs
- Go over search engines and old medium articles and check that all dead URLs re-directing correctly
~~~

* [Clarify syntax on `nym-nodes` ports on VPS setup page](https://nymtech.net/operators/nodes/vps-setup.html#configure-your-firewall), PR [\#4734](https://github.com/nymtech/nym/commit/5e6417f83788f30b2a84e4dd73d6dd9619a2bb16): Make crystal clear that the addresses and ports in operators `config.toml` must be opened using [`ufw`](https://nymtech.net/operators/nodes/vps-setup.html#configure-your-firewall) and set up as in the example below:
~~~admonish example collapsible=true title='snap of binding addresses and ports in `config.toml`'
```toml
[host]
public_ips = [
'<YOUR_PUBLIC_IPv4>'
]

[mixnet]
bind_address = '0.0.0.0:1789'

[http]
bind_address = '0.0.0.0:8080'

[mixnode]
[mixnode.verloc]
bind_address = '0.0.0.0:1790'

[entry_gateway]
bind_address = '0.0.0.0:9000'
```
~~~

### Tooling

* [Nym Harbourmaster](https://https://harbourmaster.nymtech.net/) has now several new functionalities:
    - Tab for Mixnodes
    - Tab with Charts
    - New columns with: *Moniker (node description)*, *DP delegatee*, *Accepted T&Cs* - also part of a new category 🐼😀

* Nym has a new [Token page](https://nymtech.net/about/token)

---


## `v2024.8-wispa`

- [Release binaries](https://github.com/nymtech/nym/releases/tag/nym-binaries-v2024.8-wispa)
- [Release CHANGELOG.md](https://github.com/nymtech/nym/blob/nym-binaries-v2024.8-wispa/CHANGELOG.md)
- [`nym-node`](nodes/nym-node.md) version `1.1.5`

~~~admonish example collapsible=true title='CHANGELOG.md'
- add event parsing to support cosmos_sdk > 0.50 ([#4697])
- Fix NR config compatibility ([#4690])
- Remove UserAgent constructor since it's weakly typed ([#4689])
- [bugfix]: Node_api_check CLI looked over roles on blacklisted nodes ([#4687])
- Add mixnodes to self describing api cache ([#4684])
- Move and whole bump of crates to workspace and upgrade some ([#4680])
- Remove code that refers to removed nym-network-statistics ([#4679])
- Remove nym-network-statistics ([#4678])
- Create UserAgent that can be passed from the binary to the nym api client ([#4677])
- Add authenticator ([#4667])

[#4697]: https://github.com/nymtech/nym/pull/4697
[#4690]: https://github.com/nymtech/nym/pull/4690
[#4689]: https://github.com/nymtech/nym/pull/4689
[#4687]: https://github.com/nymtech/nym/pull/4687
[#4684]: https://github.com/nymtech/nym/pull/4684
[#4680]: https://github.com/nymtech/nym/pull/4680
[#4679]: https://github.com/nymtech/nym/pull/4679
[#4678]: https://github.com/nymtech/nym/pull/4678
[#4677]: https://github.com/nymtech/nym/pull/4677
[#4667]: https://github.com/nymtech/nym/pull/4667
~~~


### Features

* [Default construct NodeRole](https://github.com/nymtech/nym/pull/4721): To preserve compatibility with newer clients interacting with older `nym-api`
~~~admonish example collapsible=true title='Testing steps performed'
 1. Reviewed the changes in the `nym-api-requests/src/models.rs` file.
 2. Verified that the `NymNodeDescription` struct includes the new `role` field with a default value set by `default_node_role`.
 3. Checked the implementation of the `default_node_role` function to ensure it returns `NodeRole::Inactive`.
 4. Ran the updated code in the sandbox environment.
 5. Monitored the sandbox environment for any issues or errors related to the changes.


 **Notes (if any):**
 The test was successful. No issues were flagged during the testing in the sandbox environment. The new default value for `NodeRole` ensures backward compatibility without causing disruptions.
~~~

* [Default construct NodeRole for backwards compatibility (apply [\#4721](https://github.com/nymtech/nym/pull/4721) on develop)](https://github.com/nymtech/nym/pull/4722)
* [Add upgrades to `nym-node` for `authenticator` changes](https://github.com/nymtech/nym/pull/4703)
~~~admonish example collapsible=true title='Testing steps performed'
 1. Reviewed the changes in the `gateway/src/error.rs` and `gateway/src/node/mod.rs` files.
 2. Verified the new error enum `AuthenticatorStartupFailure` was added to `GatewayError`.
 3. Confirmed the implementation of the `StartedAuthenticator` struct and its usage in the `start_authenticator` function.
 4. Ran the updated code in the canary environment.
 5. Monitored the canary environment for any issues or errors related to the changes.
~~~

* [Add event parsing to support `cosmos_sdk` > `0.50`](https://github.com/nymtech/nym/pull/4697)
~~~admonish example collapsible=true title='Testing steps performed'
 1. Reviewed the changes in `common/client-libs/validator-client/src/nyxd/cosmwasm_client/client_traits/signing_client.rs`, `logs.rs`, `types.rs`, and `nym-api/src/coconut/tests/mod.rs` files.
 2. Verified the addition of event parsing in the relevant functions and structs.
 3. Ensured that the `find_attribute` function correctly parses event attributes.
 4. Ran the updated code in the sandbox environment.
 5. Broadcasted transactions on the sandbox network to test the changes.
 6. Monitored the sandbox network for any malformed responses or errors after the test chain upgrade.
~~~

* [Send bandwidth status messages when connecting](https://github.com/nymtech/nym/pull/4691): When connecting to the gateway we get received the available bandwidth left. Emit a status messages for this, for consumption by the application layer.
~~~admonish example collapsible=true title='Testing steps performed'
 1. Reviewed the changes in `common/bandwidth-controller/src/event.rs`, `common/bandwidth-controller/src/lib.rs`, and `common/client-libs/gateway-client/src/client.rs` files.
 2. Verified the implementation of `BandwidthStatusMessage` enum for emitting status messages.
 3. Ensured `GatewayClient` is updated to send bandwidth status messages when connecting.
 4. Deployed the updated code on the canary environment.
 5. Connected to the gateway and checked for the emission of bandwidth status messages.
 6. Verified that the messages were correctly parsed and consumed by the application layer.
 7. Ran the VPN client to observe the parsed events.
 ~~~

* [Fix NR config compatibility](https://github.com/nymtech/nym/pull/4690): Recently we deleted the old statistics service provider. This fixes some issues where old configs didn't work with the latest changes.
    - Make NR able to read config with old keys in
    - Remove deleted config keys from NR template
~~~admonish example collapsible=true title='Testing steps performed'
 1. Reviewed the changes in the `service-providers/network-requester/src/config/mod.rs` and `service-providers/network-requester/src/config/template.rs` files.
 2. Ensured `NetworkRequester` config is able to read old keys for compatibility.
 3. Removed old and deleted config keys from the `NetworkRequester` template.
 4. Compiled the project to verify no issues or warnings appeared.
 5. Ran all tests to ensure that the changes did not affect the functionality.
 6. Validated that no leftover code from the old statistics service provider caused any issues.
 ~~~

* [Remove `UserAgent` constructor since it's weakly typed](https://github.com/nymtech/nym/pull/4689):
~~~admonish example collapsible=true title='Testing steps performed'
 1. Reviewed the changes in `common/http-api-client/src/user_agent.rs` file.
 2. Verified the removal of the `UserAgent` constructor and ensured that all instances of `UserAgent::new` are updated accordingly.
 3. Checked the implementation of `UserAgent` struct using `BinaryBuildInformation` and `BinaryBuildInformationOwned`.
 4. Deployed the updated code across different environments (QA, sandbox, and canary).
 5. Ran tests to ensure that the `UserAgent` struct functions correctly without the constructor.
 ~~~

* [Add mixnodes to self describing api cache](https://github.com/nymtech/nym/pull/4684):
    - Abstracts getting the self describing info a bit
    - Adds mixnodes to the cache refresher as well
    - Adds `role` field to the `NodeDescription` struct, to be able to distinguish between mixnodes and gateways
    - Switched to using `NodeStatusCache` instead of `ContractCache`
~~~admonish example collapsible=true title='Testing steps performed'
Called the new `/mixnodes/described` endpoint as well as the existing `/gateways/described` endpoint and verified that the data returned for each was correct based on the settings that different nodes have when they are setup.

For gateway endpoint, the “role” for now does not differentiate between entry and exit gateways, this will be implemented in the future.
~~~

* [Move and whole bump of crates to workspace and upgrade some](https://github.com/nymtech/nym/pull/4680):
    - Fix cargo warning for `default_features`
    - Move dirs 4.0 to workspace
    - Use workspace `base64` dep
    - Move `rand_chacha` and `x25519-dalek` to workspace
    - Use workspace `ed25519-dalek` dep
    - Move `itertools` to workspace deps and upgrade
    - Move a few partial deps to workspace  while preserving versions
~~~admonish example collapsible=true title='Testing steps performed'
 1. Reviewed the changes to move and upgrade crates to the workspace.
 2. Verified the updated dependencies:
    - Moved `dirs` to version 4.0 in the workspace.
    - Updated the `base64` dependency to use the workspace version.
    - Moved `rand_chacha` and `x25519-dalek` to the workspace.
    - Updated `ed25519-dalek` to use the workspace version.
    - Moved and upgraded `itertools` in the workspace.
    - Moved other partial dependencies to the workspace while preserving their versions.
 3. Ensured the `Cargo.toml` files across the project reflect these changes correctly.
 4. Compiled the entire project to check for any issues or warnings.
 5. Verified that all tests pass successfully after the changes.
 ~~~

* [Remove `nym-network-statistics`](https://github.com/nymtech/nym/pull/4678): Remove `nym-network-statistics` service provider that is no longer used.
~~~admonish example collapsible=true title='Testing steps performed'
 1. Reviewed the project to identify all references to `nym-network-statistics`.
 2. Removed all code and dependencies associated with `nym-network-statistics`.
 3. Ensured that no references to `nym-network-statistics` remain in the codebase, including comments, imports, and configuration files.
 4. Compiled the project to check for any issues or warnings.
 5. Ran all tests to ensure the removal did not affect the functionality of the project.
 ~~~


* [Remove code that refers to removed `nym-network-statistics`](https://github.com/nymtech/nym/pull/4679): Follow up to [\#4678](https://github.com/nymtech/nym/pull/4678) where all code interacting with it is removed.
~~~admonish example collapsible=true title='Testing steps performed'
 1. Reviewed the project to identify all references to `nym-network-statistics`.
 2. Removed all code and dependencies associated with `nym-network-statistics`.
 3. Ensured that no references to `nym-network-statistics` remain in the codebase, including comments, imports, and configuration files.
 4. Compiled the project to check for any issues or warnings.
 5. Ran all tests to ensure the removal did not affect the functionality of the project.
 ~~~

* [Create `UserAgent` that can be passed from the binary to the `nym-api` client](https://github.com/nymtech/nym/pull/4677):
    - Support setting `UserAgent` for the validator client
    - Support setting `UserAgent` in the SDK `MixnetClient`
     - Set `UserAgent` when getting the list of gateways and topology in
         - `nym-client`
         - `nym-socks5-client`
         - Standalone `ip-packet-router`

~~~admonish example collapsible=true title='Testing steps performed'
Used the nym-vpn-cli to test this, and we can visibly see the `UserAgent`, no issues with the comments mentioned above.

Example of the user agent sent:
`nym-client/1.1.36/x86_64-unknown-linux-gnu/e18bb70`

<img width="1435" alt="image" src="https://github.com/nymtech/nym/assets/60836166/5d4cc76f-84e6-45cb-9102-adc2b58a25d9">

Connected with no problems
~~~

* [Add `authenticator`](https://github.com/nymtech/nym/pull/4667)

### Bugfix

* [`Node_api_check.py` CLI looked over roles on blacklisted nodes](https://github.com/nymtech/nym/pull/4687): Removing/correcting this redundant function which results in unwanted error print, will resolve in the program not looking up the `roles` endpoint for blacklisted GWs, instead just ignores the role description and still return all other endpoints.

### Operators Guide updates

* [Create a guide to backup and restore `nym-node`](https://nymtech.net/operators/nodes/maintenance.html#backup-a-node), PR [\#4720](https://github.com/nymtech/nym/pull/4720)
* [Add manual IPv6 ifup/down network configuration](https://nymtech.net/operators/troubleshooting/vps-isp.html#network-configuration), PR [\#4651](https://github.com/nymtech/nym/pull/4651)
* [Extend ISP list](https://nymtech.net/operators/legal/isp-list.html)
* [Add SSL cert bot block to WSS setup](https://nymtech.net/operators/nodes/proxy-configuration.html#web-secure-socket-setup), [PR here](https://github.com/nymtech/nym/commits/develop/): WSS setup fully works!
* [Correct `HTTP API port` in bonding page](https://nymtech.net/operators/nodes/bonding.html#bond-via-the-desktop-wallet-recommended) , [PR \#4707](https://github.com/nymtech/nym/pull/4707): Change `HTTP API port` to `8080` on every `nym-node` by opening `config.toml` and making sure that your binding addresses and ports are as in the block below. Then go to desktop wallet and open the box called `Show advanced options` and make sure all your ports are set correctly (usually this means to change `HTTP api port` to `8080` for `mixnode` mode).
~~~admonish example collapsible=true title='snap of binding addresses and ports in `config.toml`'
```toml
[host]
public_ips = [
'<YOUR_PUBLIC_IPv4>'
]

[mixnet]
bind_address = '0.0.0.0:1789'

[http]
bind_address = '0.0.0.0:8080'

[mixnode]
[mixnode.verloc]
bind_address = '0.0.0.0:1790'

[entry_gateway]
bind_address = '0.0.0.0:9000'
```
~~~

* [Comment our deprecated node pages in `/docs`](https://github.com/nymtech/nym/pull/4727)
    - Fixes [issue \#4632](https://github.com/nymtech/nym/issues/4632)
* [Remove redundant syntax from the setup guide](https://github.com/nymtech/nym/pull/4682)

---

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

- [Remove the `nym-mixnode` and `nym-gateway` binaries from the CI upload builds action](https://github.com/nymtech/nym/pull/4693)
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
- [Web Secure Socket (WSS) guide and reverse proxy update](nodes/proxy-configuration.md), PR [here](https://github.com/nymtech/nym/pull/4694): A guide to setup `nym-node` in a secure fashion, using WSS via Nginx and Certbot. Landing page (reversed proxy) is updated and simplified.

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
