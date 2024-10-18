FROM rust:latest AS builder

COPY ./ /usr/src/nym
WORKDIR /usr/src/nym/nym-node

RUN cargo build --release

FROM ubuntu:24.04

WORKDIR /nym

COPY --from=builder /usr/src/nym/target/release/nym-node ./
ENTRYPOINT [ "/nym/nym-node" ]
