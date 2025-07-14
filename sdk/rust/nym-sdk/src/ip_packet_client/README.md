# Modified `nym-ip-packet-client`
This set of code is made up of functions from several crates from the `nym-vpn-client` monorepo which had to be imported and modified to avoid a circular dependency on the `nym-sdk` package for use in the  `mixnet_stream_wrapper_ipr` module, and is made up of:
- a modified version of (basically) the entire `nym-ip-packet-client`
- a set of IP Packet helper functions from the `nym-gateway-probe`
- a set of helpers & types from the `nym-connection-monitor`

All of these can be found in [`nym-vpn-client/nym-vpn-core/crates/`](https://github.com/nymtech/nym-vpn-client/tree/develop/nym-vpn-core/crates).
