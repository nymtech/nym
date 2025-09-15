#!/bin/bash

# Build and push Node Status Agent container to harbor.nymte.ch

set -e

# Configuration
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
WORKING_DIRECTORY="${SCRIPT_DIR}"
CONTAINER_NAME="node-status-agent"
REGISTRY="harbor.nymte.ch"
NAMESPACE="nym"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to display usage
usage() {
    echo "Usage: $0 <gateway-probe-git-ref>"
    echo "  gateway-probe-git-ref - Git reference (branch/tag/commit) for gateway probe"
    echo ""
    echo "Example: $0 main"
    echo "Example: $0 release/2025.11-cheddar"
    echo "Example: $0 v1.2.3"
    exit 1
}

# Parse arguments
if [ $# -ne 1 ]; then
    usage
fi

GATEWAY_PROBE_GIT_REF="$1"

# Get version from Cargo.toml
VERSION=$(grep "^version = " "${WORKING_DIRECTORY}/Cargo.toml" | sed -E 's/version = "(.*)"/\1/')
if [ -z "$VERSION" ]; then
    echo -e "${RED}Error: Could not extract version from Cargo.toml${NC}"
    exit 1
fi

# Clean up git ref for use in tag (replace / with -)
GIT_REF_SLUG="${GATEWAY_PROBE_GIT_REF//\//-}"

echo -e "${YELLOW}Building Node Status Agent${NC}"
echo -e "${YELLOW}Version: ${VERSION}${NC}"
echo -e "${YELLOW}Gateway Probe Git Ref: ${GATEWAY_PROBE_GIT_REF} (slug: ${GIT_REF_SLUG})${NC}"

# Login to Harbor
echo -e "${GREEN}Logging into Harbor...${NC}"
docker login "${REGISTRY}"

# Build the container
echo -e "${GREEN}Building container with gateway probe from ${GATEWAY_PROBE_GIT_REF}...${NC}"
# Build from repository root (two levels up from script location)
docker build \
    --build-arg GIT_REF="${GATEWAY_PROBE_GIT_REF}" \
    -f "${WORKING_DIRECTORY}/Dockerfile" \
    "${SCRIPT_DIR}/../.." \
    -t "${REGISTRY}/${NAMESPACE}/${CONTAINER_NAME}:${VERSION}-${GIT_REF_SLUG}" \
    -t "${REGISTRY}/${NAMESPACE}/${CONTAINER_NAME}:latest-${GIT_REF_SLUG}"

# Push to Harbor
echo -e "${GREEN}Pushing container to Harbor...${NC}"
docker push "${REGISTRY}/${NAMESPACE}/${CONTAINER_NAME}:${VERSION}-${GIT_REF_SLUG}"
docker push "${REGISTRY}/${NAMESPACE}/${CONTAINER_NAME}:latest-${GIT_REF_SLUG}"

echo -e "${GREEN}Successfully built and pushed ${CONTAINER_NAME}:${VERSION}-${GIT_REF_SLUG}${NC}"