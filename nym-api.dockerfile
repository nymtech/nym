# this will only work with VPN, otherwise remove the harbor part
FROM harbor.nymte.ch/dockerhub/rust:latest AS builder

COPY ./ /usr/src/nym
WORKDIR /usr/src/nym/nym-api
RUN cargo build --release

ENTRYPOINT ["/usr/src/nym/nym-api/entrypoint.sh"]
