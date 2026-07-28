# Delta: smol-core-stack — crate renamed to `nym-smol-core`

## MODIFIED Requirements

### Requirement: Transport-agnostic userspace TCP/IP stack

`nym-smol-core` SHALL provide a pure-Rust, `tokio`-async userspace TCP/IP stack driven by
a caller-supplied bidirectional transport of raw IP packets (`Vec<u8>`), with no OS
`tun` device and no elevated privileges. It MUST NOT depend on Go, a gVisor netstack,
or any FFI network stack.

#### Scenario: Stack driven by an abstract IP-packet transport
- **WHEN** a caller constructs the stack with a transport that yields inbound IP
  packets and accepts outbound IP packets
- **THEN** the stack routes application socket traffic to/from that transport without
  requiring any OS network interface or root privileges

#### Scenario: Assigned tunnel address configures the interface
- **WHEN** the stack is built with the tunnel's assigned IPv4/IPv6 address
- **THEN** the smoltcp interface is configured with those addresses and outbound
  packets carry them as source

### Requirement: smolmix refactored onto smol-core

The existing `smolmix` crate SHALL be refactored to consume `nym-smol-core` for its
stack, while preserving its existing public API.

#### Scenario: smolmix public API unchanged
- **WHEN** `smolmix` is rebuilt on top of `nym-smol-core`
- **THEN** existing `smolmix` public items (e.g. `Tunnel`, `tcp_connect`,
  `udp_socket`) continue to compile and behave as before
