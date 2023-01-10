## UNRELEASED

## [nym-connect-v1.1.5](https://github.com/nymtech/nym/tree/nym-connect-v1.1.5) (2023-01-10)

- get version number from tauri and display by @fmtabbara in https://github.com/nymtech/nym/pull/2684
- Feature/nym connect experimental software text by @fmtabbara in https://github.com/nymtech/nym/pull/2692
- NymConnect - Display service info in tooltip **1.1.5 Release** by @fmtabbara in https://github.com/nymtech/nym/pull/2704
- Feat/2130 tables update rebase by @gala1234 in https://github.com/nymtech/nym/pull/2742

## [nym-connect-v1.1.4](https://github.com/nymtech/nym/tree/nym-connect-v1.1.4) (2022-12-20)

This release contains the new opt-in Test & Earn program, and it uses a stress-tested directory of network requesters to improve reliability. It also has some bugfixes, performance improvements, and better error handling.

- nym-connect: send status messages from socks5 task to tauri backend by @octol in https://github.com/nymtech/nym/pull/1882
- socks5: rework waiting in inbound.rs by @octol in https://github.com/nymtech/nym/pull/1880
- Test&Earn by @mmsinclair in https://github.com/nymtech/nym/pull/2729

## [nym-connect-v1.1.3](https://github.com/nymtech/nym/tree/nym-connect-v1.1.3) (2022-12-13)

- socks5-client: added support for socks4a.

## [nym-connect-v1.1.2](https://github.com/nymtech/nym/tree/nym-connect-v1.1.2) (2022-12-06)

- socks5-client: fix error with client failing and disconnecting unnecessarily.

## [nym-connect-v1.1.1](https://github.com/nymtech/nym/tree/nym-connect-v1.1.1) (2022-11-29)

- socks5-client: fix multiplex concurrent connections ([#1720], [#1777])
- socks5-client: fix wait closing inbound connection until data is sent, and throttle incoming data in general ([#1772], [#1783],[#1789])
- socks5-client: fix shutting down all background workers if anyone of them panics or errors out. This fixes an issue where the nym-connect UI was showing connected even though the socks5 tunnel was non-functional. ([#1805])
- gateway-libs: fix decryping messages stored on the gateway between reconnects ([#1786])

- nymconnect: updated UI
- nymconnect: new help area
- nymconnect: listen for service errors and display on frontend

[#1720]: https://github.com/nymtech/nym/pull/1720
[#1772]: https://github.com/nymtech/nym/pull/1772
[#1777]: https://github.com/nymtech/nym/pull/1777
[#1783]: https://github.com/nymtech/nym/pull/1783
[#1786]: https://github.com/nymtech/nym/pull/1786
[#1789]: https://github.com/nymtech/nym/pull/1789
[#1805]: https://github.com/nymtech/nym/pull/1805


## [nym-connect-v1.1.0](https://github.com/nymtech/nym/tree/nym-connect-v1.1.0) (2022-11-09)

- nym-connect: rework of rewarding changes the directory data structures that describe the mixnet topology ([#1472])
- clients: add testing-only support for two more extended packet sizes (8kb and 16kb).
- native-client/socks5-client: `disable_loop_cover_traffic_stream` Debug config option to disable the separate loop cover traffic stream ([#1666])
- native-client/socks5-client: `disable_main_poisson_packet_distribution` Debug config option to make the client ignore poisson distribution in the main packet stream and ONLY send real message (and as fast as they come) ([#1664])
- native-client/socks5-client: `use_extended_packet_size` Debug config option to make the client use 'ExtendedPacketSize' for its traffic (32kB as opposed to 2kB in 1.0.2) ([#1671])
- network-requester: added additional Blockstream Green wallet endpoint to `example.allowed.list` ([#1611])
- validator-client: added `query_contract_smart` and `query_contract_raw` on `NymdClient` ([#1558])

[#1472]: https://github.com/nymtech/nym/pull/1472
[#1558]: https://github.com/nymtech/nym/pull/1558
[#1611]: https://github.com/nymtech/nym/pull/1611
[#1664]: https://github.com/nymtech/nym/pull/1664
[#1666]: https://github.com/nymtech/nym/pull/1666
[#1671]: https://github.com/nymtech/nym/pull/1671

## [nym-connect-v1.0.2](https://github.com/nymtech/nym/tree/nym-connect-v1.0.2) (2022-08-18)

### Changed

- nym-connect: "load balance" the service providers by picking a random Service Provider for each Service and storing in local storage so it remains sticky for the user ([#1540])
- nym-connect: the ServiceProviderSelector only displays the available Services, and picks a random Service Provider for Services the user has never used before ([#1540])
- nym-connect: add `local-forage` for storing user settings ([#1540])

[#1540]: https://github.com/nymtech/nym/pull/1540


## [nym-connect-v1.0.1](https://github.com/nymtech/nym/tree/nym-connect-v1.0.1) (2022-07-22)

### Added

- nym-connect: initial proof-of-concept of a UI around the socks5 client was added
- nym-connect: add ability to select network requester and gateway ([#1427])
- nym-connect: add ability to export gateway keys as JSON
- nym-connect: add auto updater

### Changed

- nym-connect: reuse config id instead of creating a new id on each connection

[#1427]: https://github.com/nymtech/nym/pull/1427
