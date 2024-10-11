# syntax=docker/dockerfile:1
FROM rust:slim

ENV RUST_LOG=debug
ENV AGENT_BIN="nym-node-status-agent"
ENV PROBE_BIN="nym-gateway-probe"

RUN mkdir /app
COPY ./$AGENT_BIN /app
COPY ./$PROBE_BIN /app

WORKDIR /app

ENTRYPOINT ["./nym-node-status-agent", "--command"]
CMD ["bla"]
