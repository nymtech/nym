#!/usr/bin/env bash

set -euo pipefail

VERSION=${1:-latest}

mkdir -p ./packages

declare -a BINARIES=(
    "./pkg/root/Applications/nymvpn.app/Contents/Resources/nymvpn"
    "./pkg/root/Applications/nymvpn.app/Contents/Resources/nymvpn-daemon"
    "./pkg/root/Applications/nymvpn.app/Contents/MacOS/nymvpn-ui"
)

sd "APP_VERSION" "${VERSION}" ./pkg/root/Applications/nymvpn.app/Contents/Info.plist
sd "APP_VERSION" "${VERSION}" ./pkg/Distribution


if [[ ! -z "${APPLICATION_SIGNING_IDENTITY:-}" ]] && [[ ! -z "${APPLE_TEAM_ID:-}" ]]; then
    for binary in "${BINARIES[@]}"
    do
        echo "Signing: ${binary}"
        codesign \
            --options runtime \
            --sign "${APPLICATION_SIGNING_IDENTITY}" \
            "${binary}"
    done
fi

pkgbuild  \
    --install-location /Applications \
    --identifier net.nymtech.vpn \
    --version "${VERSION}" \
    --scripts "./pkg/scripts" \
    --root "./pkg/root/Applications" \
    ./packages/net.nymtech.vpn.pkg

productbuild \
    --distribution "./pkg/Distribution" \
    --resources "./pkg/Resources" \
    --package-path ./packages \
    "nymvpn-${VERSION}-unsigned.pkg"

if [ ! -z "${INSTALLER_SIGNING_IDENTITY:-}" ]; then
    echo "Signing pkg"
    productsign \
        --sign "${INSTALLER_SIGNING_IDENTITY}" \
        "nymvpn-${VERSION}-unsigned.pkg" "nymvpn-${VERSION}.pkg"
else
    mv "nymvpn-${VERSION}-unsigned.pkg" "nymvpn-${VERSION}.pkg"
fi
