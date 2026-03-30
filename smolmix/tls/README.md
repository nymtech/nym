# smolmix-tls

Shared TLS setup for smolmix tunneled connections. Provides a pre-configured `TlsConnector` with webpki root certificates and convenience functions for TLS over `TcpStream`, used internally by `smolmix-hyper` and `smolmix-tungstenite`.

## Quick start

```rust
use smolmix_tls::{connect, connector, connect_with};

let tunnel = smolmix::Tunnel::new().await?;
let tcp = tunnel.tcp_connect("93.184.216.34:443".parse()?).await?;

// One-shot: TLS handshake over an existing TCP stream.
let tls_stream = connect(tcp, "example.com").await?;

// Reusable: create a connector once, use for many connections.
let tls = connector();
let stream1 = connect_with(&tls, tcp1, "a.example.com").await?;
let stream2 = connect_with(&tls, tcp2, "b.example.com").await?;
```

## API

- **`connector()`** -- create a reusable `TlsConnector` with webpki root certificates (clones cheaply via `Arc`)
- **`connect(tcp, hostname)`** -- one-shot TLS handshake (creates a fresh connector internally)
- **`connect_with(&connector, tcp, hostname)`** -- TLS handshake with a pre-built connector (avoids rebuilding the root store)

### Re-exports

Commonly-used types are re-exported so you don't need `tokio-rustls` in your `Cargo.toml`:

- `TlsStream` -- the connected TLS stream type (`tokio_rustls::client::TlsStream`)
- `TlsConnector` -- the connector type (`tokio_rustls::TlsConnector`)

## Dependencies

```toml
[dependencies]
smolmix-tls = { workspace = true }
```

This crate depends on `tokio-rustls`, `rustls`, and `webpki-roots`.
