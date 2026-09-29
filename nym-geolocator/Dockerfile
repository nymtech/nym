# this will only work with VPN, otherwise remove the harbor part
FROM harbor.nymte.ch/dockerhub/rust:latest AS builder

COPY ./ /usr/src/nym
WORKDIR /usr/src/nym

RUN cargo build --bin nym-geolocator --release

FROM harbor.nymte.ch/dockerhub/ubuntu:24.04

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /nym

COPY --from=builder /usr/src/nym/target/release/nym-geolocator ./
ENTRYPOINT ["/nym/nym-geolocator"]
