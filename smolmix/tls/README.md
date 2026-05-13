# smolmix-tls

Shared TLS setup for smolmix tunneled connections. Provides a pre-configured `TlsConnector` with webpki root certificates and convenience functions for TLS over `TcpStream`, used internally by `smolmix-hyper`.

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

`smolmix-tls` is one of three companion crates around [`smolmix`](https://crates.io/crates/smolmix). It pairs with [`smolmix-dns`](https://crates.io/crates/smolmix-dns) for hostname-based connections, or use [`smolmix-hyper`](https://crates.io/crates/smolmix-hyper) for a complete HTTP client built on top. Arrows show conceptual layering, not strict Cargo dependencies.

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

- **`connector()`**: create a reusable `TlsConnector` with webpki root certificates (clones cheaply via `Arc`)
- **`connect(tcp, hostname)`**: one-shot TLS handshake (creates a fresh connector internally)
- **`connect_with(&connector, tcp, hostname)`**: TLS handshake with a pre-built connector (avoids rebuilding the root store)

### Re-exports

Commonly-used types are re-exported so you don't need `tokio-rustls` in your `Cargo.toml`:

- `TlsStream`: the connected TLS stream type (`tokio_rustls::client::TlsStream`)
- `TlsConnector`: the connector type (`tokio_rustls::TlsConnector`)

## Resolving hostnames

The quick-start above uses a literal IP. For real hostnames, pair with [`smolmix-dns`](../dns) so resolution also goes through the tunnel (no DNS leaks to the local network):

```rust
use smolmix_dns::Resolver;
use smolmix_tls::{connect_with, connector};

let resolver = Resolver::new(&tunnel);
let addr = resolver.resolve("example.com", 443).await?
    .into_iter().next().ok_or("no addresses resolved")?;

let tcp = tunnel.tcp_connect(addr).await?;
let tls = connector();
let stream = connect_with(&tls, tcp, "example.com").await?;
```

`smolmix-tls` itself stays DNS-agnostic on purpose: it accepts any `TcpStream`, so callers with a pre-resolved address or a different resolver don't pay for `hickory-resolver`. See `examples/connect.rs` for a runnable end-to-end version.

## Dependencies

```toml
[dependencies]
smolmix = "1.21.0"
smolmix-tls = "1.21.0"
```

This crate depends on `tokio-rustls`, `rustls`, and `webpki-roots`.

## See also

- [`smolmix-dns`](../dns) for tunneled hostname resolution
- [`smolmix-hyper`](../hyper) for a complete HTTP client (DNS + TLS + HTTP) built on top of this
