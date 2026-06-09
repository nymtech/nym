# smolmix-dns

DNS resolution through the Nym mixnet. Wraps [hickory-resolver](https://docs.rs/hickory-resolver) with a `Resolver` newtype that routes all DNS queries through a smolmix `Tunnel`, preventing hostname leaks to the local network.

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

`smolmix-dns` is one of three companion crates around [`smolmix`](https://crates.io/crates/smolmix). It pairs with [`smolmix-tls`](https://crates.io/crates/smolmix-tls) for HTTPS connections, or use [`smolmix-hyper`](https://crates.io/crates/smolmix-hyper) for a complete HTTP client built on top. Arrows show conceptual layering, not strict Cargo dependencies.

## Quick start

```rust
use smolmix_dns::Resolver;

let tunnel = smolmix::Tunnel::new().await?;
let resolver = Resolver::new(&tunnel);

// Full hickory-resolver API via Deref:
let lookup = resolver.lookup_ip("example.com").await?;
for ip in lookup.iter() { println!("{ip}"); }

// Convenience one-shot:
let addrs = resolver.resolve("example.com", 443).await?;
```

## API

- **`Resolver::new(&tunnel)`**: Quad9 upstream (`9.9.9.9`)
- **`Resolver::with_config(&tunnel, config)`**: custom upstream DNS
- **`Resolver::resolve(&self, host, port)`**: convenience one-shot returning `Vec<SocketAddr>`
- **`Deref` to hickory's `Resolver`**: full `lookup_ip()`, `lookup()`, etc.
- **`resolve(&tunnel, host, port)`**: free function for quick one-shots
- **`resolver(&tunnel)`**: free function returning a `Resolver`

### Re-exports

Commonly-used hickory types are re-exported so you don't need `hickory-resolver` in your `Cargo.toml`:

- `ResolverConfig`, `LookupIp`, `ResolveError`

## Example

Clearnet-vs-mixnet DNS comparison with timing:

```sh
cargo run -p smolmix-dns --example resolve
cargo run -p smolmix-dns --example resolve -- --ipr <IPR_ADDRESS>
```

## Dependencies

```toml
[dependencies]
smolmix = "1.21.0"
smolmix-dns = "1.21.0"
```

This crate depends on `smolmix` (for the `Tunnel` type), `hickory-proto`, and `hickory-resolver`.

## See also

- [`smolmix-tls`](../tls) for TLS over the tunnel once you have a resolved address
- [`smolmix-hyper`](../hyper) for a complete HTTP client that bundles DNS + TLS + HTTP
