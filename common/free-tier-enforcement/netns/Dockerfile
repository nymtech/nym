# Environment for the free-tier network-namespace datapath tests.
# Must be run --privileged (needs NET_ADMIN + network namespaces). The repo is
# bind-mounted at /workspace and the build/test happens at run time (see run.sh),
# so this image only provides the toolchain + net tooling.
FROM rust:slim-bookworm

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        iproute2 \
        iptables \
        iputils-ping \
        ca-certificates \
        pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace
CMD ["bash"]
