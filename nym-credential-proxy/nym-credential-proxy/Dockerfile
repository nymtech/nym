FROM rust:latest AS builder

COPY ./ /usr/src/nym
WORKDIR /usr/src/nym/nym-credential-proxy/nym-credential-proxy

RUN cargo build --release

FROM ubuntu:24.04

WORKDIR /nym

COPY --from=builder /usr/src/nym/nym-credential-proxy/target/release/nym-credential-proxy ./
ENTRYPOINT [ "/nym/nym-credential-proxy" ]
