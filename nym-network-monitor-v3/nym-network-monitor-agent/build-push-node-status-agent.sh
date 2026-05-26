#!/bin/bash

# Build and push Network Monitor Agent container to harbor.nymte.ch

set -e

# Configuration
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
WORKING_DIRECTORY="${SCRIPT_DIR}"
CONTAINER_NAME="network-monitor-agent"
REGISTRY="harbor.nymte.ch"
NAMESPACE="nym"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Get version from Cargo.toml
VERSION=$(grep "^version = " "${WORKING_DIRECTORY}/Cargo.toml" | sed -E 's/version = "(.*)"/\1/')
if [ -z "$VERSION" ]; then
    echo -e "${RED}Error: Could not extract version from Cargo.toml${NC}"
    exit 1
fi

echo -e "${YELLOW}Building Network Monitor Agent${NC}"
echo -e "${YELLOW}Version: ${VERSION}${NC}"

# Login to Harbor
echo -e "${GREEN}Logging into Harbor...${NC}"
docker login "${REGISTRY}"

# Build the container
echo -e "${GREEN}Building the container...${NC}"
# Build from repository root (two levels up from script location)
docker build \
    --build-arg GIT_REF="${GATEWAY_PROBE_GIT_REF}" \
    -f "${WORKING_DIRECTORY}/Dockerfile" \
    "${SCRIPT_DIR}/../.." \
    -t "${REGISTRY}/${NAMESPACE}/${CONTAINER_NAME}:${VERSION}" \
    -t "${REGISTRY}/${NAMESPACE}/${CONTAINER_NAME}:latest"

# Push to Harbor
echo -e "${GREEN}Pushing container to Harbor...${NC}"
docker push "${REGISTRY}/${NAMESPACE}/${CONTAINER_NAME}:${VERSION}"
docker push "${REGISTRY}/${NAMESPACE}/${CONTAINER_NAME}:latest"

echo -e "${GREEN}Successfully built and pushed ${CONTAINER_NAME}:${VERSION}${NC}"