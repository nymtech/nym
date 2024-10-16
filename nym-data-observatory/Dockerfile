FROM rust:latest AS builder

COPY ./ /usr/src/nym
WORKDIR /usr/src/nym/nym-data-observatory

RUN cargo build --release

FROM ubuntu:24.04

RUN echo "Acquire::http::Pipeline-Depth 0;" > /etc/apt/apt.conf.d/99custom && \
  echo "Acquire::http::No-Cache true;" >> /etc/apt/apt.conf.d/99custom && \
  echo "Acquire::BrokenProxy    true;" >> /etc/apt/apt.conf.d/99custom

RUN apt update && apt install -yy curl

WORKDIR /nym

COPY --from=builder /usr/src/nym/target/release/nym-data-observatory ./
ENTRYPOINT [ "/nym/nym-data-observatory" ]
