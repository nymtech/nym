# nym-mixnode Changelog

## 0.3.1

* Fixed crash when directory server goes down

## 0.3.0

* cleaned up a lot of internal dependencies
* reporting version to the directory server
* printing warning on trying to bind to "localhost", "127.0.0.1" or "0.0.0.0"
* more informative error messages
* generalised identity keys
* generalised Topology handling
* started slow transition to `log` crate by `nym-client`
* start of 'MixMining'
* start of validator node

## 0.2.0

* removed the `--local` flag
* introduced `--directory` argument to support arbitrary directory servers. Leaving it out will point the node at the "https://directory.nymtech.net" alpha testnet server
* the `host` argument is now required
* IPv6 support
* mixnode version number is now shown at node start
* directory server location is now shown at node start

## 0.1.0 - Initial Release

* The bare minimum set of features required by a Nym Mixnode
