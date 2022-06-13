FROM golang:buster as go_builder
ARG BECH32_PREFIX
ARG WASMD_VERSION
RUN apt update && apt install -y git build-essential
COPY setup.sh .
RUN ./setup.sh

FROM ubuntu:20.04
COPY --from=go_builder /go/wasmd/build/nymd /root/nymd
COPY --from=go_builder /go/wasmd/build/libwasmvm*.so /root
COPY init_and_start.sh .
ENTRYPOINT ["./init_and_start.sh"]
