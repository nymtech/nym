FROM ubuntu:24.04

ARG UID=1001
ARG GID=1001
ARG NYM_CLI_GIT_TAG
ARG WASMD_VERSION
ARG RETAIN_BLOCKS

RUN apt-get update && apt-get install -y \
    curl \
    wget \
    openssl \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd -g "${GID}" nym || true
RUN useradd --create-home --no-log-init -u "${UID}" -g "${GID}" nym || true

WORKDIR /home/nym

RUN wget -O nym-cli "https://github.com/nymtech/nym/releases/download/nym-binaries-v${NYM_CLI_GIT_TAG:-2025.8-tourist}/nym-cli" || echo "nym-cli not available" && \
    wget "https://github.com/nymtech/nyxd/releases/download/${WASMD_VERSION:-v0.54.0}/nyxd-ubuntu-22.04.tar.gz" && \
    tar -zxvf nyxd-ubuntu-22.04.tar.gz && \
    mv libwasmvm.*.so /lib/ || true && \
    chmod 755 nyxd && \
    rm nyxd-ubuntu-22.04.tar.gz

# Create directories and set permissions
RUN mkdir -p .nyxd output && \
    chown -R nym:nym ./ && \
    chmod 750 .nyxd output && \
    touch .env && \
    chown nym:nym .env

VOLUME /home/nym/.nyxd
VOLUME /home/nym/output

COPY --chown=nym:nym start.sh .
RUN chmod 750 *.sh

ENV RETAIN_BLOCKS=${RETAIN_BLOCKS}

ENTRYPOINT ["./start.sh"]
CMD ["genesis"]