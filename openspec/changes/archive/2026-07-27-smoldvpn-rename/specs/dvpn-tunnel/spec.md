# Delta: dvpn-tunnel — crate renamed to `smoldvpn`

## MODIFIED Requirements

### Requirement: Userspace WireGuard datapath with boringtun

`smoldvpn` SHALL implement the WireGuard datapath using `boringtun` in userspace,
with no OS `tun` device and no root. It MUST NOT use `defguard_wireguard_rs`,
`wireguard-go`, or any Go/FFI engine. Each WireGuard peer's public key and preshared
key SHALL be taken from the session's registration output.

#### Scenario: Bring up a tunnel with no OS interface
- **WHEN** a tunnel is connected with valid per-hop WireGuard configuration
- **THEN** encryption/decryption occurs in-process via `boringtun` and no OS network
  interface is created and no elevated privileges are required

#### Scenario: Peer configured from registration
- **WHEN** a WireGuard peer is configured
- **THEN** its public key and preshared key are those returned by gateway
  registration
