# rust-libp2p-nym

This repo contains an example implementation of a libp2p transport using the Nym mixnet. It relies on the ChainSafe's fork of libp2p: https://github.com/ChainSafe/rust-libp2p

## Requirements

- Rust 1.68.2
- `Protoc` protobuf compiler. On Debian/Ubuntu distributed via `apt` as `protobuf-compiler` & on Arch/Manjaro via AUR as `[python-protobuf-compiler](https://aur.archlinux.org/packages/python-protobuf-compiler)`. 

## Usage

To instantiate a libp2p swarm using the transport:

```rust
use libp2p::core::{muxing::StreamMuxerBox, transport::Transport};
use libp2p::swarm::{keep_alive::Behaviour, SwarmBuilder};
use libp2p::{identity, PeerId};
use nym_sdk::mixnet::MixnetClient;
use rust_libp2p_nym::transport::NymTransport;
use rust_libp2p_nym::test_utils::create_nym_client;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    info!("Local peer id: {local_peer_id:?}");

    let nym_id = rand::random::<u64>().to_string();
    let nym_client = MixnetClient::connect_new().await.unwrap();
    let transport = NymTransport::new(nym_client, local_key.clone()).await?;
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

## Chat example

To run the libp2p chat example, run the following in one terminal:
```bash
cargo run --example libp2p_chat
# Local peer id: PeerId("12D3KooWLukBu6q2FerWPFhFFhiYaJkhn2sBmceh9UCaXe6hJf5D")
# Listening on "/nym/FhtkzizQg2JbZ19kGkRKXdjV2QnFbT5ww88ZAKaD4nkF.7Remi4UVYzn1yL3qYtEcQBGh6tzTYxMdYB4uqyHVc5Z4@62F81C9GrHDRja9WCqozemRFSzFPMecY85MbGwn6efve"
```

In another terminal, run ping again, passing the Nym multiaddress printed previously:
```bash
cargo run --example libp2p_chat -- /nym/FhtkzizQg2JbZ19kGkRKXdjV2QnFbT5ww88ZAKaD4nkF.7Remi4UVYzn1yL3qYtEcQBGh6tzTYxMdYB4uqyHVc5Z4@62F81C9GrHDRja9WCqozemRFSzFPMecY85MbGwn6efve
# Local peer id: PeerId("12D3KooWNsuRwG6DHnFJCDR8B3zdvja6xLcfnbtKCsQWJ8eppyWC")
# Dialed /nym/FhtkzizQg2JbZ19kGkRKXdjV2QnFbT5ww88ZAKaD4nkF.7Remi4UVYzn1yL3qYtEcQBGh6tzTYxMdYB4uqyHVc5Z4@62F81C9GrHDRja9WCqozemRFSzFPMecY85MbGwn6efve
# Listening on "/nym/2oiRW5C9ivyF3Bo3Gpm4H9EqSKH7A6GpcrRRwVSDVUQ9.EajgCnhzimsP6KskUwKcEj8VFCmHR78s2J6FHWcZ4etR@Fo4f4SQLdoyoGkFae5TpVhRVoXCF8UiypLVGtGjujVPf"
```

You should see that the nodes connected and sent messages to each other:
```bash
# 2023-08-10T14:06:28.116Z INFO  libp2p_chat                > Got message: 'hello world' with id: 37393732353836333838333537303637303237 from peer: 12D3KooWB6k8ZGDF44N4FMRhgVBNihwk1wMYSumosxiZq9pUTbAz
```
