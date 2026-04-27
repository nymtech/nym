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

Tunnel lifecycle (call once at startup):

| Export | Purpose |
|--------|---------|
| `setupMixTunnel(opts)` | Initialise mixnet client + smoltcp stack |
| `disconnectMixTunnel()` | Graceful shutdown |

## Architecture

```
Browser main thread                Worker thread                  WASM (Rust)
-------------------                -------------                  -----------
                                   setupMixTunnel(opts) --------> Nym client connect
                                                                  smoltcp stack init
                                                                  IPR tunnel setup

mixFetch(url, init)
  Comlink RPC ----------------------> wasmFetch() --------------> DNS -> TCP -> TLS -> HTTP
  <-- Response object --------------- { body, status, headers } <-- httparse response

new MixSocket(url)
  postMessage(ws-connect) ----------> wasmMixSocket() ----------> DNS -> TCP -> TLS -> WS upgrade
  <-- postMessage(ws-event:open) ---- onEvent("open") <---------- HTTP 101 + spawn recv loop

ws.send(data)
  postMessage(ws-send) -------------> wasmWsSend() -------------> channel -> frame encode -> stream

                                      onEvent("text", data) <---- frame decode <- stream
  <-- postMessage(ws-event:text) ----'

ws.close()
  postMessage(ws-close) ------------> wasmWsClose() ------------> channel -> close frame
  <-- postMessage(ws-event:close) --- onEvent("close") <--------- close reply received
```

Two communication channels on the same Worker:
- **Comlink** (request-response) for `mixFetch` — transparent async function calls
- **Raw postMessage** (bidirectional events) for `MixSocket` — Comlink can't handle server-push

## Module map

```
src/
  lib.rs        WASM exports, handle maps, background tasks
  tunnel.rs     WasmTunnel: Nym client + smoltcp + connection pool
  bridge.rs     Mixnet <-> smoltcp bridge (packet forwarding loop)
  reactor.rs    smoltcp poll loop (timers, socket events)
  device.rs     smoltcp Device impl (virtual NIC)
  ipr.rs        IP Packet Router protocol (SOCKS-like exit relay)
  lp.rs         Length-prefixed framing for IPR messages
  dns.rs        DNS resolution over tunnel UDP (simple-dns, ~100 lines)
  fetch.rs      HTTP orchestrator: DNS -> TCP -> TLS -> HTTP
  http.rs       HTTP/1.1 request/response codec (httparse, ~150 lines)
  tls.rs        TLS setup (futures-rustls + webpki-roots)
  ws.rs         WebSocket frame codec + upgrade + WsConnection (RFC 6455)
  error.rs      FetchError enum

internal-dev/
  worker.js       Web Worker: Comlink (fetch) + postMessage (WS)
  mix-socket.js   MixSocket class (drop-in WebSocket replacement)
  index.html      Dev test harness UI
  index.js        Test harness logic (HTTP, WS, stress tests)
  headless.js     Headless test runner (Playwright-compatible)
  webpack.config.js

tests/
  tests/smoke.spec.mjs    Playwright: tunnel setup smoke test
  tests/suite.spec.mjs    Playwright: full test suite
```

## Build

```sh
# Debug build (faster, larger, console_log enabled)
make build-debug

# Release build (optimised, smaller)
make build-release

# Dev server (webpack, hot reload)
cd internal-dev && npm run start
```

## TODO

### Crate split

The majority of this crate is transport infrastructure (tunnel, bridge,
reactor, device, IPR, DNS, TCP, TLS). The public API surface is thin —
just `mixFetch` and `mixSocket` with their JS interop.

Proposed split:

```
smolmix-wasm-core    tunnel, bridge, reactor, device, ipr, lp,
                     dns, tls, http, ws, error
                     (pure Rust, no wasm_bindgen exports)

smolmix-fetch        mixFetch + setupMixTunnel/disconnect
                     (thin WASM wrapper over core)

smolmix-socket       mixSocket + wsSend/wsClose
                     (thin WASM wrapper over core)

smolmix-wasm         re-exports both (convenience crate)
```

Benefits:
- Users who only need fetch don't pull in WS code (and vice versa)
- Core can be tested natively (no wasm32 target required)
- Cleaner dependency graph for downstream crates

### Other

- [ ] Connection pool sharing between fetch and WS (currently fetch-only)
- [ ] `MixSocket` sub-protocol negotiation test
- [ ] Binary message round-trip test (echo server)
- [ ] `binaryType = "arraybuffer"` end-to-end test
- [ ] Transferable optimisation for large WS messages (zero-copy postMessage)
- [ ] Proper close handshake timeout (currently waits indefinitely for close reply)
- [ ] Reconnection support in `MixSocket` (auto-reconnect with backoff)
- [ ] `wsSend` / `wsClose` export names — consider `mixSocketSend` / `mixSocketClose` for consistency
- [ ] `FormData`, `Blob`, `ReadableStream` body types in `mixFetch`
- [ ] `Headers` object and `[string, string][]` header formats in `mixFetch`
