# rust-libp2p-nym

This repo contains an implementation of a libp2p transport using the Nym mixnet.

## Requirements

- Rust 1.68.2

## Usage

To instantiate a libp2p swarm using the transport:

```rust
use libp2p::core::{muxing::StreamMuxerBox, transport::Transport};
use libp2p::swarm::{keep_alive::Behaviour, SwarmBuilder};
use libp2p::{identity, PeerId};
use rust_libp2p_nym::transport::NymTransport;
use rust_libp2p_nym::test_utils::create_nym_client;
use std::error::Error;
use testcontainers::clients;
use tracing::{info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    info!("Local peer id: {local_peer_id:?}");

    let nym_id = rand::random::<u64>().to_string();
    let docker_client = clients::Cli::default();
    let (_nym_container, dialer_uri) = create_nym_client(&docker_client, &nym_id);
    let transport = NymTransport::new(&dialer_uri, local_key.clone()).await?;
    let _swarm = SwarmBuilder::with_tokio_executor(
        transport
            .map(|a, _| (a.0, StreamMuxerBox::new(a.1)))
            .boxed(),
        Behaviour::default(),
        local_peer_id,
    )
    .build();
    Ok(())
}
```

See `examples/ping.rs` for a full usage example.

Alternatively, you can connect to a known Nym client directly instead of using a local Dockerized client by passing in the client's websockets endpoint to `NymTransport::new()`, which is `ws://127.0.0.1:1977` by default.

## Tests

Install `protoc`. On Ubuntu/Debian, run: `sudo apt-get install
protobuf-compiler`.

Ensure that docker is installed on the machine where the tests need to
be run. 

Then, run the following as usual.

```
./build-docker.sh
```
This builds the docker image for the nym service locally.

### Notes on Docker

* The Docker image is a *local* image and we are not pushing this
  anywhere. The tag is reflective of the version of the binaries that
  we are downloading from github releases for the nym client

### Writing New Tests

In order to abstract away the `nym-client` instantiation, we rely on the
[`testcontainers`
crate](https://docs.rs/testcontainers/latest/testcontainers/index.html) that
launches the service for us. Since there are no publicly maintained versions of
this, we use our own Dockerfile.

In order to create a single service, developers can use the following code
snippet.

```rust
let dialer_uri: String = Default::default();
rust_libp2p_nym::new_nym_client!(nym_id, dialer_uri);
```

One can create as many of these as needed, limited only by the server resources.

For more usage patterns, look at `src/transport.rs`. Note that if the code terminates
in a non-clean way, you might have to kill the running docker containers
manually using `docker rm -f $ID".

## Ping example

To run the libp2p ping example, run the following in one terminal:
```bash
cargo run --example ping
# Local peer id: PeerId("12D3KooWLukBu6q2FerWPFhFFhiYaJkhn2sBmceh9UCaXe6hJf5D")
# Listening on "/nym/FhtkzizQg2JbZ19kGkRKXdjV2QnFbT5ww88ZAKaD4nkF.7Remi4UVYzn1yL3qYtEcQBGh6tzTYxMdYB4uqyHVc5Z4@62F81C9GrHDRja9WCqozemRFSzFPMecY85MbGwn6efve"
```

In another terminal, run ping again, passing the Nym multiaddress printed previously:
```bash
cargo run --example ping -- /nym/FhtkzizQg2JbZ19kGkRKXdjV2QnFbT5ww88ZAKaD4nkF.7Remi4UVYzn1yL3qYtEcQBGh6tzTYxMdYB4uqyHVc5Z4@62F81C9GrHDRja9WCqozemRFSzFPMecY85MbGwn6efve
# Local peer id: PeerId("12D3KooWNsuRwG6DHnFJCDR8B3zdvja6xLcfnbtKCsQWJ8eppyWC")
# Dialed /nym/FhtkzizQg2JbZ19kGkRKXdjV2QnFbT5ww88ZAKaD4nkF.7Remi4UVYzn1yL3qYtEcQBGh6tzTYxMdYB4uqyHVc5Z4@62F81C9GrHDRja9WCqozemRFSzFPMecY85MbGwn6efve
# Listening on "/nym/2oiRW5C9ivyF3Bo3Gpm4H9EqSKH7A6GpcrRRwVSDVUQ9.EajgCnhzimsP6KskUwKcEj8VFCmHR78s2J6FHWcZ4etR@Fo4f4SQLdoyoGkFae5TpVhRVoXCF8UiypLVGtGjujVPf"
```

You should see that the nodes connected and pinged each other:
```bash
# Mar 30 22:56:36.400  INFO ping: BehaviourEvent: Event { peer: PeerId("12D3KooWGf2oYd6U2nrLzfDrN9zxsjSQjPsMh2oDJPUQ9hiHMNtf"), result: Ok(Ping { rtt: 1.06836675s }) }
```
```bash
# Mar 30 22:56:35.595  INFO ping: BehaviourEvent: Event { peer: PeerId("12D3KooWMd5ak31DXuZq7x1JuFSR6toA5RDQrPaHrfXEhy7vqqpC"), result: Ok(Pong) }
```

In order to run the ping example with vanilla libp2p, which uses tcp, pass the
`--features vanilla` flag to the example and follow the instructions on the
rust-libp2p project as usual.

```bash
RUST_LOG=ping=debug cargo run --examples ping --feature vanilla
```

```bash
RUST_LOG=ping=debug cargo run --examples ping --feature vanilla -- "/ip4/127.0.0.1/tcp/$PORT"
```
