# smolmix-wasm

Drop-in browser networking over the Nym mixnet. Routes HTTP and WebSocket
traffic through a mixnet tunnel, giving web applications network-level
privacy without changing application code.

## Public API

Three WASM exports that mirror the browser's native networking surface:

| Browser API | smolmix export | Description |
|-------------|---------------|-------------|
| `fetch()` | `mixFetch(url, init)` | HTTP/HTTPS request-response |
| `new WebSocket()` | `mixWebSocket(url, protocols, onEvent)` | WebSocket (WS/WSS) |
| (no direct browser equivalent) | `mixDNS(hostname)` | DNS-only hostname lookup over UDP/IPR (no TCP/TLS) |

## Browser-shape header shim

`mixFetch` injects a small set of default request headers when the caller
hasn't set them. Many CDNs (cloudflare's bot management) and host policies
(wikimedia's User-Agent policy) reject requests that lack browser-canonical
headers.

| Header | Default | Why |
|--------|---------|-----|
| `User-Agent` | `Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36` | A recent Chrome-on-Linux string. Pinned so it doesn't drift between builds. |
| `Accept` | `text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8` | Matches what Chrome sends for a navigation request. |
| `Accept-Language` | `en-US,en;q=0.9` | Single locale. Set this explicitly per-request if your app is localised. |
| `Accept-Encoding` | `identity` | hyper 1.x in our wasm build has no decompressor; advertising `gzip, deflate, br` would surface compressed bytes to the caller un-decoded. Body-correctness over wire-shape. |

Canonical strings: `DEFAULT_USER_AGENT`, `DEFAULT_ACCEPT`,
`DEFAULT_ACCEPT_LANGUAGE`, `DEFAULT_ACCEPT_ENCODING` in `src/http.rs`.

The shim does NOT attempt full browser impersonation. TLS fingerprint (JA3),
HTTP/2 (we're HTTP/1.1 only), and header ordering are all distinguishable
from a real Chrome request.

To override a default for a single request:

```ts
await mixFetch('https://example.com', {
  headers: { 'User-Agent': 'my-app/1.0' },
});
```

## Arch

```text
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
- WASM exports (`lib.rs`, `mixfetch.rs`, `mixwebsocket.rs`, `mixdns.rs`) - the surface JS calls into

### Tuning

The JS `setupMixTunnel(opts)` shape accepts the following optional fields for
timeouts, buffer sizes, and protocol limits. All have sensible defaults; only
override when you have a concrete reason.

| Field             | Default | Notes                                                          |
|-------------------|---------|----------------------------------------------------------------|
| `connectTimeoutMs`| `60000` | IPR connect handshake timeout                                  |
| `dnsTimeoutMs`    | `30000` | DNS query timeout (per primary/fallback attempt)               |
| `tcpKeepaliveMs`  | `10000` | TCP keepalive probe interval                                   |
| `tcpBufferSize`   | `65535` | Per-TCP-stream RX/TX buffer; capped at `u16::MAX`              |
| `maxRedirects`    | `5`     | `mixFetch` redirect chain depth before bail                    |

On the Rust side these live in `TunnelOpts::tuning: TuningOpts`. The builder
exposes them flat (`.connect_timeout(d)`, `.tcp_buffer_size(n)`, etc.) so
callers don't see the grouping.

### Feature flags

The crate is split into three user-facing cargo features matching the JS entry
points. Default builds enable all three; downstream TS SDK packages can opt
into a subset to drop the corresponding implementation + native deps from the
wasm binary.

| Feature    | JS export       | Pulls                                              |
|------------|-----------------|----------------------------------------------------|
| `dns`      | `mixDNS`        | (nothing extra; DNS resolver is always compiled)   |
| `fetch`    | `mixFetch`      | rustls TLS stack + hyper HTTP/1.1 client           |
| `websocket`| `mixWebSocket`  | rustls TLS stack + async-tungstenite               |

Build a `dns`-only client:

```sh
cargo build --target wasm32-unknown-unknown --no-default-features --features dns
```

Build a `fetch`-only client (no WebSocket, no `mixDNS` JS export):

```sh
cargo build --target wasm32-unknown-unknown --no-default-features --features fetch
```

`fetch` and `websocket` share the TLS stack (rustls + rustls-rustcrypto + webpki-roots);
enabling both is roughly the same wasm size as either alone plus the hyper +
async-tungstenite specifics.

### Debug logging

`debug_log!` and `debug_error!` (in `util.rs`) wrap `nym_wasm_utils::console_log!` /
`console_error!` behind the `debug` cargo feature. Tunnel start/shutdown and the
IPR connect handshake stay unconditional; everything else is silent in release.

`make build-debug` enables the feature automatically (it builds with
`--features debug`). `make build-release-opt` leaves it off, so release
artefacts ship no verbose logging.

### Cryptography

TLS terminates inside the WASM client, so we need a pure-Rust rustls
crypto provider. [`rustls-rustcrypto`](https://github.com/RustCrypto/rustls-rustcrypto) as
the only viable option: the underlying RustCrypto AEADs were
[audited by NCC Group in 2020](https://www.nccgroup.com/research/public-report-rustcrypto-aesgcm-and-chacha20pluspoly1305-implementation-review/)
with no findings, while the rustls integration glue is `0.0.2-alpha`.
`src/tls.rs` restricts negotiation to AEAD-only suites with forward-secret
key exchange.

## Build

```sh
make build              # plain release wasm-pack build
make build-debug        # dev profile, verbose console logs on
make build-release-opt  # release + wasm-opt -Oz
make dev                # build-debug then start internal-dev webpack
```

## Summary diagram

```text
              JS caller
                 |
       +---------+---------+--------------+
       v                   v              v
  mixFetch            mixWebSocket    mixDNS
  (mixfetch.rs)      (mixwebsocket.rs) (mixdns.rs)
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
`mixWebSocket`) is content travelling inside that single gateway WSS as
Sphinx-packed bytes.
