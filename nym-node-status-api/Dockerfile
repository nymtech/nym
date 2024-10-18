FROM rust:latest AS builder

COPY ./ /usr/src/nym
WORKDIR /usr/src/nym/nym-node-status-api

RUN cargo build --release

FROM ubuntu:24.04

WORKDIR /nym

COPY --from=builder /usr/src/nym/target/release/nym-node-status-api ./
ENTRYPOINT [ "/nym/nym-node-status-api" ]
