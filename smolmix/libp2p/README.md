# smolmix-libp2p

libp2p `Transport` implementation that routes connections through the Nym mixnet. Because smolmix provides real TCP streams (via a user-space smoltcp stack), libp2p's standard noise encryption and yamux multiplexing work out of the box -- no custom `Connection`, `Substream`, or message ordering needed.

## Quick start

```rust
use libp2p::{noise, yamux, SwarmBuilder};
use libp2p::core::upgrade::Version;
use libp2p::core::Transport;
use smolmix_libp2p::SmolmixTransport;

let tunnel = smolmix::Tunnel::new().await?;

let mut swarm = SwarmBuilder::with_new_identity()
    .with_tokio()
    .with_other_transport(|keypair| {
        SmolmixTransport::new(&tunnel)
            .upgrade(Version::V1)
            .authenticate(noise::Config::new(keypair).expect("noise config"))
            .multiplex(yamux::Config::default())
            .boxed()
    })?
    .with_behaviour(|_| libp2p::ping::Behaviour::default())?
    .build();

swarm.dial("/ip4/1.2.3.4/tcp/12345".parse::<libp2p::Multiaddr>()?)?;
```

## API

- **`SmolmixTransport::new(&tunnel)`** -- create a transport backed by a smolmix `Tunnel`

### Transport trait

- **`dial(addr, opts)`** -- parse multiaddr, resolve DNS if needed, TCP connect through the tunnel
- **`listen_on(id, addr)`** -- returns `MultiaddrNotSupported` (dial-only, no inbound)
- **`poll(cx)`** -- always `Pending` (no listener events)

### Supported multiaddrs

| Multiaddr | Behaviour |
|---|---|
| `/ip4/<addr>/tcp/<port>` | Direct TCP connect through tunnel |
| `/ip6/<addr>/tcp/<port>` | Direct TCP connect through tunnel |
| `/dns4/<host>/tcp/<port>` | DNS resolved through tunnel, then TCP connect |
| `/dns/<host>/tcp/<port>` | DNS resolved through tunnel, then TCP connect |

Trailing components (like `/p2p/<peer_id>`) are ignored by the transport and handled by the swarm.

## Examples

Two examples demonstrate the full workflow — a clearnet listener and a mixnet dialer:

```sh
# Terminal 1: start a standard libp2p node (clearnet TCP, noise + yamux)
cargo run -p smolmix-libp2p --example listener
# → prints: /ip4/127.0.0.1/tcp/12345/p2p/12D3KooW...

# Terminal 2: dial it through the Nym mixnet
cargo run -p smolmix-libp2p --example ping -- /ip4/<YOUR_IP>/tcp/<PORT>/p2p/<PEER_ID>
cargo run -p smolmix-libp2p --example ping -- --ipr <IPR_ADDRESS> /ip4/.../tcp/.../p2p/...
```

The listener sees a connection arriving from the exit gateway's IP — it has no idea the dialer is on the mixnet.

## Limitations

- **Dial-only** -- listening requires IPR listener support (future work)
- **No TLS in transport** -- libp2p uses noise for encryption; TLS would be redundant

## Dependencies

```toml
[dependencies]
smolmix-libp2p = { workspace = true }
```

This crate depends on `smolmix`, `smolmix-dns`, `libp2p` (0.56), and `tokio-util` (for the `Compat` bridge between tokio and futures I/O traits).
