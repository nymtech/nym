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
- WasmTcpStream / WasmUdpSocket / PooledConn (`stream.rs`) - `futures::io::AsyncRead + AsyncWrite` adapters over smoltcp sockets
- WASM exports (`lib.rs`, `mixfetch.rs`, `mixsocket.rs`) - the surface JS calls into

### Debug logging

`debug_log!` and `debug_error!` (in `util.rs`) wrap `nym_wasm_utils::console_log!` /
`console_error!` behind the `debug` cargo feature. Tunnel start/shutdown and the
IPR connect handshake stay unconditional; everything else is silent in release.

`make build-debug` enables the feature automatically (it builds with
`--features rustcrypto,debug`). `make build-release-opt` leaves it off, so
release artefacts ship no verbose logging.

## Build

```sh
make build              # plain release wasm-pack build
make build-debug        # dev profile, verbose console logs on
make build-release-opt  # release + wasm-opt -Oz
make dev                # build-debug then start internal-dev webpack
```

## Summary diagram

```
              JS caller
                 |
       +---------+---------+--------------+
       v                   v              v
  mixFetch            mixSocket       mixResolve
  (mixfetch.rs)      (mixsocket.rs)   (mixdns.rs)
       |                   |              |
       v                   v              v
  fetch::fetch       fetch::new_      dns::resolve
                     connection +     (dns.rs)
                     async_tungst.
       \                   |              /
        \                  v             /
         '-> WasmTcpStream / WasmUdpSocket  (stream.rs)
                            |
                            v  smoltcp socket buffer
                  +-------- smoltcp::Interface::poll() (reactor.rs)
                  |
                  v IP packet
            WasmDevice.tx_queue  (device.rs)
                  |
                  v drained 5ms
            bridge::start_bridge  (bridge.rs)
                  |
                  v
            ipr::send_ip_packet  (ipr.rs)
                  |
                  v  LP-framed DataRequest
            ClientInput::send  (upstream, nym-wasm-client-core)
                  |
                  v  Sphinx-packed
            JSWebsocket::new  -> WebSocket::open -> web_sys::WebSocket::new
                  (common/wasm/utils/src/websocket/mod.rs:58)
                  |
                  v
            Single wss:// to chosen gateway

  (Separately, at startup + on TopologyRefresher tick:)
            nym_http_api_client::ClientBuilder
              -> reqwest -> web_sys::fetch
              (common/client-core/src/init/helpers.rs:155)
              |
              v
            HTTPS GET https://validator.nymtech.net/...
```

Everything else (TLS handshakes, HTTP/1.1 requests, WebSocket frames in
`mixSocket`) is *content* travelling **inside** that single gateway WSS as
Sphinx-packed bytes.
