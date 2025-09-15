#!/bin/bash

# Build and push Node Status API container to harbor.nymte.ch

set -e

# Configuration
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
WORKING_DIRECTORY="${SCRIPT_DIR}"
CONTAINER_NAME="node-status-api"
REGISTRY="harbor.nymte.ch"
NAMESPACE="nym"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to display usage
usage() {
    echo "Usage: $0 [pg|sqlite|both]"
    echo "  pg     - Build and push PostgreSQL version"
    echo "  sqlite - Build and push SQLite version"
    echo "  both   - Build and push both versions (default)"
    exit 1
}

# Parse arguments
DB_TYPE="${1:-both}"

if [[ ! "$DB_TYPE" =~ ^(pg|sqlite|both)$ ]]; then
    usage
fi

# Get version from Cargo.toml
VERSION=$(grep "^version = " "${WORKING_DIRECTORY}/Cargo.toml" | sed -E 's/version = "(.*)"/\1/')
if [ -z "$VERSION" ]; then
    echo -e "${RED}Error: Could not extract version from Cargo.toml${NC}"
    exit 1
fi
echo -e "${YELLOW}Version: ${VERSION}${NC}"

# Login to Harbor
echo -e "${GREEN}Logging into Harbor...${NC}"
docker login "${REGISTRY}"

# Function to build and push
build_and_push() {
    local db_type=$1
    local dockerfile="Dockerfile-${db_type}"
    
    echo -e "${GREEN}Building ${db_type} container...${NC}"
    # Build from repository root (two levels up from script location)
    docker build -f "${WORKING_DIRECTORY}/${dockerfile}" "${SCRIPT_DIR}/../.." \
        -t "${REGISTRY}/${NAMESPACE}/${CONTAINER_NAME}:${VERSION}-${db_type}" \
        -t "${REGISTRY}/${NAMESPACE}/${CONTAINER_NAME}:latest-${db_type}"
    
    echo -e "${GREEN}Pushing ${db_type} container to Harbor...${NC}"
    docker push "${REGISTRY}/${NAMESPACE}/${CONTAINER_NAME}:${VERSION}-${db_type}"
    docker push "${REGISTRY}/${NAMESPACE}/${CONTAINER_NAME}:latest-${db_type}"
    
    echo -e "${GREEN}Successfully built and pushed ${CONTAINER_NAME}:${VERSION}-${db_type}${NC}"
}

# Build based on selection
case "$DB_TYPE" in
    pg)
        build_and_push "pg"
        ;;
    sqlite)
        build_and_push "sqlite"
        ;;
    both)
        build_and_push "pg"
        echo ""
        build_and_push "sqlite"
        ;;
esac

echo -e "${GREEN}All builds completed successfully!${NC}"