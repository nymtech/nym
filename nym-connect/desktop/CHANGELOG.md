# Changelog

## [Unreleased]

## [v1.1.21-kitkat] (2023-09-12)

- NC - Handle failure when config is too old ([#3847])

[#3847]: https://github.com/nymtech/nym/issues/3847

## [v1.1.20-twix] (2023-09-05)

- nym-connect directory error handling ([#3830])
- NC - it should not be possible to toggle speedy mode while the connection is active ([#3816])

[#3830]: https://github.com/nymtech/nym/pull/3830
[#3816]: https://github.com/nymtech/nym/issues/3816

## [v1.1.19-snickers] (2023-08-29)

- NymConnect sometimes fails to connect because the gateway it fetches from the validator-api to use is running an old version (of the gateway binary) ([#3788])

[#3788]: https://github.com/nymtech/nym/issues/3788

## [1.1.18] (2023-08-22)

- refactor(nc-desktop): use userdata storage to save user gateway&sp ([#3723])

[#3723]: https://github.com/nymtech/nym/pull/3723

## [1.1.17] (2023-08-16)

- Add a "Send us your feedback" section in NC (on the main screen)  to collect user feedback using Sentry ([#3619])
- NC native android - deploy on FDroid ([#3483])

[#3619]: https://github.com/nymtech/nym/issues/3619
[#3483]: https://github.com/nymtech/nym/issues/3483

## [v1.1.16] (2023-08-08)

- Uncouple network-requester <-> gateway in nym-connect and harbourmaster ([#3472])

[#3472]: https://github.com/nymtech/nym/issues/3472

## [v1.1.15] (2023-07-25)

- NC Desktop - remove sentry DSN from code ([#3694])
- NC - Add Alephium wallet in the supported app list ([#3681])

[#3694]: https://github.com/nymtech/nym/issues/3694
[#3681]: https://github.com/nymtech/nym/issues/3681

## [v1.1.14] (2023-07-04)

- Nym connect fails to start when encountering an old config version ([#3588])
- NC desktop - apps section adjustments + add monero integration ([#2977])
- nym-connect: use different service provider directory when medium toggle enabled ([#3617])
- Fix medium toggle in nym-connect ([#3590])
- [bugfix] NC: load old gateway configuration if we're not registering ([#3586])
- nym-connect: medium speed setting ([#3585])

[#3588]: https://github.com/nymtech/nym/issues/3588
[#2977]: https://github.com/nymtech/nym/issues/2977
[#3617]: https://github.com/nymtech/nym/pull/3617
[#3590]: https://github.com/nymtech/nym/pull/3590
[#3586]: https://github.com/nymtech/nym/pull/3586
[#3585]: https://github.com/nymtech/nym/pull/3585

## [v1.1.13] (2023-06-20)

- NymConnect - add sentry.io reporting ([#3421])

[#3421]: https://github.com/nymtech/nym/issues/3421

## [v1.1.12] (2023-03-07)

- NymConnect - Update display for selected Service Provider ([#3116])

[#3116]: https://github.com/nymtech/nym/issues/3116

## [v1.1.11] (2023-02-28)

- NC - add the option to manually select and use a specific Service Provider ([#2953])

[#2953]: https://github.com/nymtech/nym/issues/2953

## [v1.1.10] (2023-02-21)

- NC - add logs window for troubleshooting ([#2951])

[#2951]: https://github.com/nymtech/nym/issues/2951

## [nym-connect-v1.1.9](https://github.com/nymtech/nym/tree/nym-connect-v1.1.9) (2023-02-14)

- Button animations ([#2949])
- add effect when the button is clicked ([#2947])
- UI to select gateways based on some performance criteria by checking gateways' routing score from nym-api ([#2942])
- client health check when connecting ([#2859])
- allow user to select own gateway ([#2952])

[#2952]: https://github.com/nymtech/nym/issues/2952
[#2949]: https://github.com/nymtech/nym/issues/2949
[#2947]: https://github.com/nymtech/nym/issues/2947
[#2942]: https://github.com/nymtech/nym/issues/2942
[#2859]: https://github.com/nymtech/nym/issues/2859

## [nym-connect-v1.1.8](https://github.com/nymtech/nym/tree/nym-connect-v1.1.8) (2023-01-31)

- Add supported apps in the menu + update guide ([#2868])
- Copy changes to remove the dropdown: ([#2777])

[#2868]: https://github.com/nymtech/nym/issues/2868
[#2777]: https://github.com/nymtech/nym/issues/2777

## [nym-connect-v1.1.7](https://github.com/nymtech/nym/tree/nym-connect-v1.1.7) (2023-01-24)

- Remove test and earn ([#2865])

[#2865]: https://github.com/nymtech/nym/issue/2865

## [nym-connect-v1.1.6](https://github.com/nymtech/nym/tree/nym-connect-v1.1.6) (2023-01-17)

- part (1) show gateway status on the UI if the gateway is not live, is overloaded or is slow ([#2824])

[#2824]: https://github.com/nymtech/nym/pull/2824

## [nym-connect-v1.1.5](https://github.com/nymtech/nym/tree/nym-connect-v1.1.5) (2023-01-10)

- get version number from tauri and display by @fmtabbara in https://github.com/nymtech/nym/pull/2684
- Feature/nym connect experimental software text by @fmtabbara in https://github.com/nymtech/nym/pull/2692
- NymConnect - Display service info in tooltip **1.1.5 Release** by @fmtabbara in https://github.com/nymtech/nym/pull/2704

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
- validator-client: added `query_contract_smart` and `query_contract_raw` on `NyxdClient` ([#1558])

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
