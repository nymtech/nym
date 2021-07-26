FROM golang:buster as go_builder
ARG BECH32_PREFIX
ENV BECH32_PREFIX $BECH32_PREFIX
ARG WASMD_VERSION
ENV WASMD_VERSION $WASMD_VERSION
RUN apt update && apt install -y git build-essential
COPY setup.sh .
RUN ./setup.sh

FROM ubuntu:20.04
ARG BECH32_PREFIX
ENV BECH32_PREFIX $BECH32_PREFIX
COPY --from=go_builder /go/wasmd/build/nymd /root/nymd
COPY --from=go_builder /go/wasmd/build/libwasmvm.so /root/libwasmvm.so
COPY init_and_start.sh .
ENTRYPOINT ["./init_and_start.sh"]
