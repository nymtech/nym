FROM rust:latest AS builder

COPY ./ /usr/src/nym
WORKDIR /usr/src/nym/nym-network-monitor
RUN cargo build --release

FROM locustio/locust
EXPOSE 8089
COPY --from=0 /usr/src/nym/target/release/nym-network-monitor /bin/nym-network-monitor
COPY --from=0 /usr/src/nym/nym-network-monitor/locustfile.py locustfile.py
COPY --from=0 /usr/src/nym/nym-network-monitor/entrypoint.sh entrypoint.sh
COPY --from=0 /usr/src/nym/envs/mainnet.env mainnet.env

ENTRYPOINT ["./entrypoint.sh"]
