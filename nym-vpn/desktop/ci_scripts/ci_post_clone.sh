#!/bin/sh
#  ci_post_clone.sh
cd /nym-vpn/desktop && \
curl https://sh.rustup.rs -sSf | sh && \
cargo install cargo-deb;
cargo install --force cargo-make;
cargo install sd;
cargo install ripgrep;
cargo install cargo-about;
cargo install cargo-generate-rpm;
brew install protobuf;
cargo make pkg;




