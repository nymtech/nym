test: clippy-all cargo-test wasm fmt
test-all: test cargo-test-expensive
no-clippy: build cargo-test wasm fmt
happy: fmt clippy-happy test
clippy-all: clippy-main clippy-main-examples clippy-all-contracts clippy-all-wallet clippy-all-connect clippy-all-connect-android clippy-all-wasm-client
clippy-happy: clippy-happy-main clippy-happy-contracts clippy-happy-wallet clippy-happy-connect clippy-happy-connect-android
cargo-test: test-main test-contracts test-wallet test-connect test-connect-android
cargo-test-expensive: test-main-expensive test-contracts-expensive test-wallet-expensive test-connect-expensive
build: build-contracts build-wallet build-main build-main-examples build-connect build-connect-android build-wasm-client
fmt: fmt-main fmt-contracts fmt-wallet fmt-connect fmt-connect-android fmt-wasm-client

clippy-happy-main:
	cargo clippy

clippy-happy-contracts:
	cargo clippy --manifest-path contracts/Cargo.toml --target wasm32-unknown-unknown

clippy-happy-wallet:
	cargo clippy --manifest-path nym-wallet/Cargo.toml

clippy-happy-connect:
	cargo clippy --manifest-path nym-connect/Cargo.toml

clippy-happy-connect-android:
	cargo clippy --manifest-path nym-connect-android/src-tauri/Cargo.toml

clippy-main:
	cargo clippy --workspace -- -D warnings

clippy-main-examples:
	cargo clippy --workspace --examples -- -D warnings

clippy-wasm:
	cargo clippy --manifest-path clients/webassembly/Cargo.toml --target wasm32-unknown-unknown --workspace -- -D warnings


clippy-all-contracts:
	cargo clippy --workspace --manifest-path contracts/Cargo.toml --all-features --target wasm32-unknown-unknown -- -D warnings

clippy-all-wallet:
	cargo clippy --workspace --manifest-path nym-wallet/Cargo.toml --all-features -- -D warnings

clippy-all-connect:
	cargo clippy --workspace --manifest-path nym-connect/Cargo.toml --all-features -- -D warnings

clippy-all-connect-android:
	cargo clippy --workspace --manifest-path nym-connect-android/src-tauri/Cargo.toml --all-features -- -D warnings

clippy-all-wasm-client:
	cargo clippy --workspace --manifest-path clients/webassembly/Cargo.toml --all-features --target wasm32-unknown-unknown -- -D warnings

test-main:
	cargo test --workspace

test-main-expensive:
	cargo test --workspace -- --ignored

test-contracts:
	cargo test --manifest-path contracts/Cargo.toml --all-features

test-contracts-expensive:
	cargo test --manifest-path contracts/Cargo.toml --all-features -- --ignored

test-wallet:
	cargo test --manifest-path nym-wallet/Cargo.toml --all-features

test-wallet-expensive:
	cargo test --manifest-path nym-wallet/Cargo.toml --all-features -- --ignored

test-connect:
	cargo test --manifest-path nym-connect/Cargo.toml --all-features

test-connect-expensive:
	cargo test --manifest-path nym-connect/Cargo.toml --all-features -- --ignored

test-connect-android:
	cargo test --manifest-path nym-connect-android/src-tauri/Cargo.toml --all-features

test-connect-android-expensive:
	cargo test --manifest-path nym-connect-android/src-tauri/Cargo.toml --all-features -- --ignored

build-main:
	cargo build --workspace

build-main-examples:
	cargo build --workspace --examples

build-contracts:
	cargo build --manifest-path contracts/Cargo.toml --workspace

build-wallet:
	cargo build --manifest-path nym-wallet/Cargo.toml --workspace

build-connect:
	cargo build --manifest-path nym-connect/Cargo.toml --workspace

build-connect-android:
	cargo build --manifest-path nym-connect-android/src-tauri/Cargo.toml --workspace

build-explorer-api:
	cargo build --manifest-path explorer-api/Cargo.toml --workspace

build-wasm-client:
	cargo build --manifest-path clients/webassembly/Cargo.toml --workspace --target wasm32-unknown-unknown

build-nym-cli:
	cargo build --release --manifest-path tools/nym-cli/Cargo.toml

fmt-main:
	cargo fmt --all

fmt-contracts:
	cargo fmt --manifest-path contracts/Cargo.toml --all

fmt-wallet:
	cargo fmt --manifest-path nym-wallet/Cargo.toml --all

fmt-connect:
	cargo fmt --manifest-path nym-connect/Cargo.toml --all

fmt-connect-android:
	cargo fmt --manifest-path nym-connect-android/src-tauri/Cargo.toml --all

fmt-wasm-client:
	cargo fmt --manifest-path clients/webassembly/Cargo.toml --all

wasm:
	RUSTFLAGS='-C link-arg=-s' cargo build --manifest-path contracts/Cargo.toml --release --target wasm32-unknown-unknown

mixnet-opt: wasm
	cd contracts/mixnet && make opt

generate-typescript:
	cd tools/ts-rs-cli && cargo run && cd ../..
	yarn types:lint:fix

run-validator-tests:
	cd nym-api/tests/functional_test && yarn test:qa
