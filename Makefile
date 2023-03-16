# Default target
all: test

test: clippy-all cargo-test wasm fmt
test-no-mobile: clippy-all-no-mobile cargo-test-no-mobile wasm fmt-no-mobile

test-all: test cargo-test-expensive
test-all-no-mobile: test-no-mobile cargo-test-expensive

no-clippy: build cargo-test wasm fmt
no-clippy-no-mobile: build-no-mobile cargo-test-no-mobile wasm fmt-no-mobile

happy: fmt clippy-happy test
happy-no-mobile: fmt-no-mobile clippy-happy-no-mobile test-no-mobile

clippy-all: clippy-all-no-mobile clippy-all-connect-mobile
clippy-all-no-mobile: clippy-main clippy-main-examples clippy-all-contracts clippy-all-wallet clippy-all-connect clippy-all-wasm-client

clippy-happy: clippy-happy-no-mobile clippy-happy-connect-mobile
clippy-happy-no-mobile: clippy-happy-main clippy-happy-contracts clippy-happy-wallet clippy-happy-connect

cargo-test: cargo-test-no-mobile
build: build-no-mobile

#
# Main workspace
#

clippy-happy-main:
	cargo clippy

clippy-main:
	cargo clippy --workspace -- -D warnings

clippy-main-examples:
	cargo clippy --workspace --examples -- -D warnings

test-main:
	cargo test --workspace

test-main-expensive:
	cargo test --workspace -- --ignored

build-main:
	cargo build --workspace

build-main-examples:
	cargo build --workspace --examples

fmt-main:
	cargo fmt --all

cargo-test-no-mobile: test-main
cargo-test-expensive: test-main-expensive
build-no-mobile: build-main build-main-examples
fmt: fmt-main

#
# Contracts
#

clippy-happy-contracts:
	cargo clippy --manifest-path contracts/Cargo.toml --workspace --target wasm32-unknown-unknown

clippy-all-contracts:
	cargo clippy --manifest-path contracts/Cargo.toml --workspace --all-features --target wasm32-unknown-unknown -- -D warnings

test-contracts:
	cargo test --manifest-path contracts/Cargo.toml --all-features

test-contracts-expensive:
	cargo test --manifest-path contracts/Cargo.toml --all-features -- --ignored

build-contracts:
	cargo build --manifest-path contracts/Cargo.toml --workspace

fmt-contracts:
	cargo fmt --manifest-path contracts/Cargo.toml --all

cargo-test-no-mobile: test-contracts
cargo-test-expensive: test-contracts-expensive
build-no-mobile: build-contracts
fmt: fmt-contracts

#
# nym-wallet
#

clippy-happy-wallet:
	cargo clippy --manifest-path nym-wallet/Cargo.toml

clippy-all-wallet:
	cargo clippy --workspace --manifest-path nym-wallet/Cargo.toml --all-features -- -D warnings

test-wallet:
	cargo test --manifest-path nym-wallet/Cargo.toml --all-features

test-wallet-expensive:
	cargo test --manifest-path nym-wallet/Cargo.toml --all-features -- --ignored

build-wallet:
	cargo build --manifest-path nym-wallet/Cargo.toml --workspace

fmt-wallet:
	cargo fmt --manifest-path nym-wallet/Cargo.toml --all

cargo-test-no-mobile: test-wallet
cargo-test-expensive: test-wallet-expensive
build-no-mobile: build-wallet
fmt: fmt-wallet

#
# nym-connect desktop
#

clippy-happy-connect:
	cargo clippy --manifest-path nym-connect/desktop/Cargo.toml

clippy-all-connect:
	cargo clippy --workspace --manifest-path nym-connect/desktop/Cargo.toml --all-features -- -D warnings

test-connect:
	cargo test --manifest-path nym-connect/desktop/Cargo.toml --all-features

test-connect-expensive:
	cargo test --manifest-path nym-connect/desktop/Cargo.toml --all-features -- --ignored

build-connect:
	cargo build --manifest-path nym-connect/desktop/Cargo.toml --workspace

fmt-connect:
	cargo fmt --manifest-path nym-connect/desktop/Cargo.toml --all

cargo-test-no-mobile: test-connect
cargo-test-expensive: test-connect-expensive
build-no-mobile: build-connect
fmt: fmt-connect

#
# nym-connect mobile
#

clippy-happy-connect-mobile:
	cargo clippy --manifest-path nym-connect/mobile/src-tauri/Cargo.toml

clippy-all-connect-mobile:
	cargo clippy --workspace --manifest-path nym-connect/mobile/src-tauri/Cargo.toml --all-features -- -D warnings

test-connect-mobile:
	cargo test --manifest-path nym-connect/mobile/src-tauri/Cargo.toml --all-features

test-connect-mobile-expensive:
	cargo test --manifest-path nym-connect/mobile/src-tauri/Cargo.toml --all-features -- --ignored

build-connect-mobile:
	cargo build --manifest-path nym-connect/mobile/src-tauri/Cargo.toml --workspace

fmt-connect-mobile:
	cargo fmt --manifest-path nym-connect/mobile/src-tauri/Cargo.toml --all

cargo-test: test-connect-mobile
build: build-connect-mobile
fmt: fmt-connect-mobile

#
# nym-client-wasm
#

clippy-wasm:
	cargo clippy --manifest-path clients/webassembly/Cargo.toml --target wasm32-unknown-unknown --workspace -- -D warnings

clippy-all-wasm-client:
	cargo clippy --workspace --manifest-path clients/webassembly/Cargo.toml --all-features --target wasm32-unknown-unknown -- -D warnings

build-wasm-client:
	cargo build --manifest-path clients/webassembly/Cargo.toml --workspace --target wasm32-unknown-unknown

fmt-wasm-client:
	cargo fmt --manifest-path clients/webassembly/Cargo.toml --all

build-no-mobile: build-wasm-client
fmt: fmt-wasm-client

#
# Convenience targets for crates that are already part of the main workspace
#

build-explorer-api:
	cargo build -p explorer-api

build-nym-cli:
	cargo build -p nym-cli --release

#
# Misc
#

wasm:
	RUSTFLAGS='-C link-arg=-s' cargo build --manifest-path contracts/Cargo.toml --release --target wasm32-unknown-unknown
	wasm-opt -Os contracts/target/wasm32-unknown-unknown/release/vesting_contract.wasm -o contracts/target/wasm32-unknown-unknown/release/vesting_contract.wasm
	wasm-opt -Os contracts/target/wasm32-unknown-unknown/release/mixnet_contract.wasm -o contracts/target/wasm32-unknown-unknown/release/mixnet_contract.wasm

mixnet-opt: wasm
	cd contracts/mixnet && make opt

generate-typescript:
	cd tools/ts-rs-cli && cargo run && cd ../..
	yarn types:lint:fix

run-api-tests:
	cd nym-api/tests/functional_test && yarn test:qa
