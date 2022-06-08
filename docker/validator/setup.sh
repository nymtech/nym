#!/bin/sh

git clone https://github.com/CosmWasm/wasmd.git
cd wasmd
git checkout "${WASMD_VERSION}"
mkdir build
go build \
    -o build/nymd -mod=readonly -tags "netgo,ledger" \
    -ldflags "-X github.com/cosmos/cosmos-sdk/version.Name=nymd \
    -X github.com/cosmos/cosmos-sdk/version.AppName=nymd \
    -X github.com/CosmWasm/wasmd/app.NodeDir=.nymd \
    -X github.com/cosmos/cosmos-sdk/version.Version=${WASMD_VERSION} \
    -X github.com/cosmos/cosmos-sdk/version.Commit=${WASMD_COMMIT_HASH} \
    -X github.com/CosmWasm/wasmd/app.Bech32Prefix=${BECH32_PREFIX} \
    -X 'github.com/cosmos/cosmos-sdk/version.BuildTags=netgo,ledger'" \
    -trimpath ./cmd/wasmd
find .. -type f -name 'libwasm*.so' -exec cp {} build \;
