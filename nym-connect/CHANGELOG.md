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