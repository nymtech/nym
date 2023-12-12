#!/bin/sh
#  ci_post_clone.sh
cd /Volumes/workspace/repository/nym-vpn/desktop && \
curl --proto '=https' --tlsv1.2 --retry 10 --retry-connrefused -fsSL "https://sh.rustup.rs" | sh -s -- --default-toolchain none -y &&
source "$HOME/.cargo/env";
cargo install cargo-deb;
cargo install --force cargo-make;
cargo install sd;
cargo install ripgrep;
cargo install cargo-about;
cargo install cargo-generate-rpm;
brew install protobuf;
APPLICATION_SIGNING_IDENTITY="Developer ID Application: Nym Technologies SA (VW5DZLFHM5)" \
INSTALLER_SIGNING_IDENTITY="3rd Party Mac Developer Installer: Nym Technologies SA (VW5DZLFHM5)"  \
APPLE_TEAM_ID=VW5DZLFHM5 \
cargo make pkg;




