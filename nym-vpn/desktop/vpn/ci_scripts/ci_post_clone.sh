#!/bin/sh
#  ci_post_clone.sh
cd /Volumes/workspace/repository/nym-vpn/desktop && \
curl https://sh.rustup.rs -sSf | sh && \
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




