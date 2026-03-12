# Single-stage Dockerfile for Nym localnet
# Builds: nym-node, nym-nym-api
# Target: Apple Container Runtime with host networking

# syntax=docker/dockerfile:1.4
FROM rust:1.91.1 AS builder

# Install runtime dependencies including Go for wireguard-go
#RUN apt update && apt install -y \
#    python3 \
#    python3-pip \
#    netcat-openbsd \
#    jq \
#    iproute2 \
#    net-tools \
#    wireguard-tools \
#    golang-go \
#    git \
#    && rm -rf /var/lib/apt/lists/*

RUN apt update && apt install -y \
    iproute2 \
    iptables \
    netcat-openbsd \
    net-tools \
    wireguard-tools \
    golang-go \
    git \
    jq \
    sqlite3 \
    && rm -rf /var/lib/apt/lists/*

# Install wireguard-go (userspace WireGuard implementation)
RUN git clone https://git.zx2c4.com/wireguard-go && \
    cd wireguard-go && \
    make && \
    cp wireguard-go /usr/local/bin/ && \
    cd .. && \
    rm -rf wireguard-go

WORKDIR /usr/src/nym

###############################################################
# 1. Copy only top-level manifests (always stable)
###############################################################
COPY Cargo.toml Cargo.lock ./

###############################################################
# 2. Copy *full workspace* into a temp folder
###############################################################
COPY . /tmp/fullsrc


###############################################################
# 3. Recreate directory structure for all workspace crates
#    by finding Cargo.toml files inside the container (for dependency caching)
###############################################################
RUN set -eux; \
    cd /usr/src/nym; \
    mkdir -p /usr/src/nym; \
    # find every Cargo.toml except the top-level one \
    find /tmp/fullsrc -name Cargo.toml ! -path "/tmp/fullsrc/Cargo.toml" | while read path; do \
        rel="${path#/tmp/fullsrc/}"; \
        dir="$(dirname "$rel")"; \
        mkdir -p "$dir"; \
        cp "$path" "$dir/"; \
    done

###############################################################
# 4. Dummy build (dependencies only)
###############################################################
RUN echo "fn main() {}" > dummy.rs

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/usr/src/nym/target \
    cargo build --release --locked || true

###############################################################
# 5. Copy REAL workspace sources into place
###############################################################
RUN cp -a /tmp/fullsrc/. /usr/src/nym/

###############################################################
# 6. Final build
###############################################################
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/usr/src/nym/target \
    cargo build --release --locked \
        -p nym-node \
        -p nym-api \
        -p nym-gateway-probe

# Move binaries to /usr/local/bin for easy access
RUN --mount=type=cache,target=/usr/local/cargo/registry \
        --mount=type=cache,target=/usr/local/cargo/git \
        --mount=type=cache,target=/usr/src/nym/target \
        mv /usr/src/nym/target/release/nym-node /usr/local/bin/ && \
        mv /usr/src/nym/target/release/nym-api /usr/local/bin/ && \
        mv /usr/src/nym/target/release/nym-gateway-probe /usr/local/bin/


WORKDIR /nym

# Default command
CMD ["nym-node", "--help"]
