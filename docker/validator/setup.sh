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
    -X github.com/cosmos/cosmos-sdk/version.Commit=1920f80d181adbeaedac1eeea1c1c6e1704d3e49 \
    -X github.com/CosmWasm/wasmd/app.Bech32Prefix=${BECH32_PREFIX} \
    -X 'github.com/cosmos/cosmos-sdk/version.BuildTags=netgo,ledger'" \
    -trimpath ./cmd/wasmd
WASMVM_SO=$(ldd build/nymd | grep libwasmvm.so | awk '{ print $3 }')
cp "${WASMVM_SO}" build/
