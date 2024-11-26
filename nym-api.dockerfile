FROM rust:latest AS builder

COPY ./ /usr/src/nym
WORKDIR /usr/src/nym/nym-api
RUN cargo build --release

ENTRYPOINT ["/usr/src/nym/nym-api/entrypoint.sh"]
