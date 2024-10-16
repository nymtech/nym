FROM rust:latest AS builder

COPY ./ /usr/src/nym
WORKDIR /usr/src/nym/nym-vpn-api/vpn-api-server

RUN cargo build --release

FROM ubuntu:24.04

WORKDIR /nym

COPY --from=builder /usr/src/nym/nym-vpn-api/target/release/nym-vpn-api ./
ENTRYPOINT [ "/nym/nym-vpn-api" ]
