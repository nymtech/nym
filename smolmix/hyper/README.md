# smolmix-hyper

HTTP client routing all traffic through the Nym mixnet. Wraps [hyper-util](https://docs.rs/hyper-util)'s `Client` with a newtype that handles DNS resolution, TCP connections, and TLS — all through a smolmix `Tunnel`.

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

- **`Client::new(&tunnel)`** — create an HTTP client (body type: `Empty<Bytes>`, suitable for GET)
- **`Deref` to hyper-util's `Client`** — full `request()`, `get()`, etc.
- **`SmolmixConnector::new(&tunnel)`** — for custom body types (e.g. POST with `Full<Bytes>`)
- **`client(&tunnel)`** — free function returning a `Client`

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
cargo run -p smolmix-hyper --example get -- --ipr <IPR_ADDRESS>
```

## Dependencies

```toml
[dependencies]
smolmix-hyper = { workspace = true }
```

This crate depends on `smolmix` and `smolmix-dns` (DNS resolution goes through the tunnel automatically).
