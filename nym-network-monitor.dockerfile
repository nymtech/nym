# this will only work with VPN, otherwise remove the harbor part
FROM harbor.nymte.ch/dockerhub/rust:latest AS builder

COPY ./ /usr/src/nym
WORKDIR /usr/src/nym/nym-network-monitor
RUN cargo build --release

FROM locustio/locust
EXPOSE 8089
EXPOSE 8080
COPY --from=builder /usr/src/nym/target/release/nym-network-monitor /bin/nym-network-monitor
COPY --from=builder /usr/src/nym/nym-network-monitor/locustfile.py locustfile.py
COPY --from=builder /usr/src/nym/nym-network-monitor/entrypoint.sh entrypoint.sh
COPY --from=builder /usr/src/nym/envs envs

ENTRYPOINT ["./entrypoint.sh"]
