# smolmix-wasm

Drop-in browser networking over the Nym mixnet. Routes HTTP and WebSocket
traffic through a mixnet tunnel, giving web applications network-level
privacy without changing application code.

## Public API

Two WASM exports that mirror the browser's native networking surface:

| Browser API | smolmix export | Description |
|-------------|---------------|-------------|
| `fetch()` | `mixFetch(url, init)` | HTTP/HTTPS request-response |
| `new WebSocket()` | `mixSocket(url, protocols, onEvent)` | WebSocket (WS/WSS) |

## Arch

```
                          WasmTunnel
              +---------- tunnel.rs -----------+
              |                                |
              |  Owns: smoltcp stack, Nym      |
              |  client, connection pool,      |
              |  DNS cache, origin locks       |
              +--------------------------------+
                     |            |
              TCP/UDP sockets    |
              (futures::io)      |
                     |           |
                     v           v
              +-----------+  +-----------+  +-----------+
              |  Reactor  |  |  Bridge   |  | Nym Client|
              | reactor.rs|  | bridge.rs |  | (base     |
              |           |  |           |  |  client)  |
              +-----------+  +-----------+  +-----------+
                     |           |               |
                     v           v               |
              +-----------+  +-------+           |
              |  smoltcp  |  |  IPR  |           |
              | Interface |  |ipr.rs |           |
              +-----------+  +-------+           |
                     |           |               |
                     v           |               |
              +-----------+     |                |
              |  Device   |<----+                |
              | device.rs |     |                |
              | (virtual  |     v                |
              |   NIC)    |  LP frames           |
              +-----------+  + SURBs             |
              rx[] / tx[]       |                |
                                +--------->------+
                                     mixnet
```

### Component walkthrough

- Device (`device.rs`) - the virtual network interface card
- Reactor (`reactor.rs`) - the smoltcp poll loop
- Bridge (`bridge.rs`) - shuttles packets between the device and the mixnet
- IPR (`ipr.rs`) - IP Packet Router protocol layer
- WasmTcpStream / WasmUdpSocket (`tunnel.rs`) - `futures::io::AsyncRead + AsyncWrite` adapters over smoltcp sockets

## Build

```sh
# Debug build
make build-debug

# Release build
make build-release

# Dev server (webpack, hot reload)
cd internal-dev && npm run start
```

## TODO

### Crate split

The majority of this crate is transport infrastructure (tunnel, bridge,
reactor, device, IPR, DNS, TCP, TLS). The public API surface is thin -
just `mixFetch` and `mixSocket` with JS interop.

```
smolmix-wasm-core    tunnel, bridge, reactor, device, ipr,
                     dns, tls, http, error
                     (pure Rust, no wasm_bindgen exports)

smolmix-fetch        mixFetch + setupMixTunnel/disconnect
                     (thin WASM wrapper over core)

smolmix-socket       mixSocket + wsSend/wsClose
                     (thin WASM wrapper over core)

smolmix-wasm         re-exports both (convenience crate)
```
