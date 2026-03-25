#!/usr/bin/env bash
#
# Downloads the latest timestamped Mozilla CA root certificate bundle from
# curl.se and places it in the sslhelpers package for embedding into the binary.
#
# Usage:
#   ./scripts/update-root-certs.sh                     # uses latest available bundle
#   ./scripts/update-root-certs.sh 2025-12-02          # uses a specific dated bundle
#
# Run this before the SDK version bump script.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_DIR="${SCRIPT_DIR}/../internal/sslhelpers"
OUTPUT_FILE="${OUTPUT_DIR}/cacert.pem"

# Determine which bundle to fetch
if [[ $# -ge 1 ]]; then
    DATE="$1"
else
    # Fetch the main page to find the latest dated bundle
    LATEST_DATE=$(curl -sL https://curl.se/docs/caextract.html \
        | grep -oP 'cacert-\K[0-9]{4}-[0-9]{2}-[0-9]{2}(?=\.pem)' \
        | sort -r | head -1)
    if [[ -z "$LATEST_DATE" ]]; then
        echo "ERROR: Could not determine latest bundle date from curl.se"
        exit 1
    fi
    DATE="$LATEST_DATE"
    echo "Latest bundle date: ${DATE}"
fi

PEM_URL="https://curl.se/ca/cacert-${DATE}.pem"
SHA_URL="${PEM_URL}.sha256"

echo "Downloading ${PEM_URL} ..."
TMPFILE=$(mktemp /tmp/cacert-XXXXXX.pem)
trap "rm -f ${TMPFILE}" EXIT

curl -sL -o "${TMPFILE}" "${PEM_URL}"

# Verify SHA256
EXPECTED_SHA=$(curl -sL "${SHA_URL}" | awk '{print $1}')
ACTUAL_SHA=$(sha256sum "${TMPFILE}" | awk '{print $1}')

if [[ "${EXPECTED_SHA}" != "${ACTUAL_SHA}" ]]; then
    echo "ERROR: SHA256 mismatch!"
    echo "  expected: ${EXPECTED_SHA}"
    echo "  actual:   ${ACTUAL_SHA}"
    exit 1
fi
echo "SHA256 verified: ${ACTUAL_SHA}"

CERT_COUNT=$(grep -c 'BEGIN CERTIFICATE' "${TMPFILE}")

cp "${TMPFILE}" "${OUTPUT_FILE}"
echo "Done. Placed ${CERT_COUNT} certificates (bundle date: ${DATE}) at ${OUTPUT_FILE}"
