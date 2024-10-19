FROM rust:latest AS builder

COPY ./ /usr/src/nym
WORKDIR /usr/src/nym/nym-data-observatory

RUN cargo build --release

FROM ubuntu:24.04

RUN apt update && apt install -yy curl ca-certificates

WORKDIR /nym

COPY --from=builder /usr/src/nym/target/release/nym-data-observatory ./
ENTRYPOINT [ "/nym/nym-data-observatory" ]
