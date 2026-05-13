# smolmix-hyper

HTTP client routing all traffic through the Nym mixnet. Wraps [hyper-util](https://docs.rs/hyper-util)'s `Client` with a newtype that handles DNS resolution, TCP connections, and TLS, all through a smolmix `Tunnel`.

## Workspace layout

```text
                  smolmix-hyper
                   (top-level)
                   /          \
                  v            v
            smolmix-dns ←→ smolmix-tls
            (resolution)    (encryption)
                   \          /
                    v        v
                     smolmix
                    (tunnel)
```

`smolmix-hyper` is the top-level convenience crate, bundling DNS, TLS, and HTTP. For lower-level control, use [`smolmix-dns`](https://crates.io/crates/smolmix-dns) and [`smolmix-tls`](https://crates.io/crates/smolmix-tls) directly over a [`smolmix`](https://crates.io/crates/smolmix) tunnel. Arrows show conceptual layering, not strict Cargo dependencies.

## Quick start

```rust
use smolmix_hyper::{Client, Request, EmptyBody, BodyExt};
use bytes::Bytes;

let tunnel = smolmix::Tunnel::new().await?;
let client = Client::new(&tunnel);

let req = Request::get("https://example.com")
    .header("Host", "example.com")
    .body(EmptyBody::<Bytes>::new())?;
let resp = client.request(req).await?;
let body = resp.into_body().collect().await?.to_bytes();
```

## API

- **`Client::new(&tunnel)`**: create an HTTP client (body type: `Empty<Bytes>`, suitable for GET)
- **`Deref` to hyper-util's `Client`**: full `request()`, `get()`, etc.
- **`SmolmixConnector::new(&tunnel)`**: for custom body types (e.g. POST with `Full<Bytes>`)
- **`client(&tunnel)`**: free function returning a `Client`

### Re-exports

Commonly-used types are re-exported so you don't need `hyper`, `http-body-util`, or `bytes` in your `Cargo.toml`:

- `Request`, `Response`, `StatusCode`, `Uri`
- `BodyExt`, `EmptyBody` (alias for `http_body_util::Empty`)
- `bytes` (the crate, for `Bytes`)

### POST and custom body types

The `Client` newtype is typed for `Empty<Bytes>` (GET requests). For POST, use `SmolmixConnector` directly:

```rust
use smolmix_hyper::SmolmixConnector;
use hyper_util::client::legacy::Client;
use http_body_util::Full;

let connector = SmolmixConnector::new(&tunnel);
let client: Client<SmolmixConnector, Full<Bytes>> =
    Client::builder(TokioExecutor::new()).build(connector);

let req = Request::post("https://httpbin.org/post")
    .header("Content-Type", "application/json")
    .body(Full::new(Bytes::from(r#"{"key": "value"}"#)))?;
let resp = client.request(req).await?;
```

## Examples

Clearnet-vs-mixnet HTTP comparisons with timing:

```sh
cargo run -p smolmix-hyper --example get    # HTTPS GET
cargo run -p smolmix-hyper --example post   # HTTP POST with JSON body
cargo run -p smolmix-hyper --example get -- --ipr <IPR_ADDRESS> # If you want to use a specific IPR on an Exit Gateway
```

## Dependencies

```toml
[dependencies]
smolmix = "1.21.0"
smolmix-hyper = "1.21.0"
```

This crate depends on `smolmix`, `smolmix-dns` (DNS resolution through the tunnel), and `smolmix-tls` (webpki roots, TLS handshake).

## See also

If you want lower-level control over individual steps rather than the full HTTP client:

- [`smolmix-dns`](../dns) for tunneled DNS resolution only
- [`smolmix-tls`](../tls) for TLS only (bring your own `TcpStream` from the tunnel)
