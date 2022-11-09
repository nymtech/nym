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