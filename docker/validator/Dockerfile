FROM ubuntu:24.04

ARG WASMD_VERSION
ARG NYM_CLI_GIT_TAG
ARG RETAIN_BLOCKS

RUN apt-get update && apt-get install -y \
    ca-certificates \
    jq \
    curl \
    wget \
    vim \
    openssl \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /root

RUN wget -O nym-cli "https://github.com/nymtech/nym/releases/download/nym-binaries-v${NYM_CLI_GIT_TAG:-2025.8-tourist}/nym-cli" && \
    chmod 755 nym-cli || echo "nym-cli not available"

RUN wget "https://github.com/nymtech/nyxd/releases/download/${WASMD_VERSION:-v0.54.3}/nyxd-ubuntu-22.04.tar.gz" && \
    tar -zxvf nyxd-ubuntu-22.04.tar.gz && \
    mv libwasmvm.x86_64.so /lib/x86_64-linux-gnu/ && \
    chmod 755 nyxd && \
    rm nyxd-ubuntu-22.04.tar.gz

RUN mkdir -p "/root/.nyxd/config" "/root/output"
RUN chmod 755 "/root/.nyxd" "/root/.nyxd/config" "/root/output"

VOLUME /root/.nyxd
VOLUME /root/output

RUN touch .env

COPY start.sh .
RUN chmod 755 *.sh

ENV RETAIN_BLOCKS=${RETAIN_BLOCKS}

ENTRYPOINT ["./start.sh"]
CMD ["genesis"]