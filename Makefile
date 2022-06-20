test: build clippy-all cargo-test wasm fmt
test-all: test cargo-test-expensive
no-clippy: build cargo-test wasm fmt
happy: fmt clippy-happy test
clippy-all: clippy-all-main clippy-all-contracts clippy-all-wallet clippy-all-connect
clippy-happy: clippy-happy-main clippy-happy-contracts clippy-happy-wallet clippy-happy-connect
cargo-test: test-main test-contracts test-wallet test-connect
cargo-test-expensive: test-main-expensive test-contracts-expensive test-wallet-expensive test-connect-expensive
build: build-contracts build-wallet build-main build-connect
fmt: fmt-main fmt-contracts fmt-wallet fmt-connect

clippy-happy-main:
	cargo clippy

clippy-happy-contracts:
	cargo clippy --manifest-path contracts/Cargo.toml --target wasm32-unknown-unknown

clippy-happy-wallet:
	cargo clippy --manifest-path nym-wallet/Cargo.toml

clippy-happy-connect:
	cargo clippy --manifest-path nym-connect/Cargo.toml

clippy-all-main:
	cargo clippy --workspace --all-features -- -D warnings

clippy-all-contracts:
	cargo clippy --workspace --manifest-path contracts/Cargo.toml --all-features --target wasm32-unknown-unknown -- -D warnings

clippy-all-wallet:
	cargo clippy --workspace --manifest-path nym-wallet/Cargo.toml --all-features -- -D warnings

clippy-all-connect:
	cargo clippy --workspace --manifest-path nym-connect/Cargo.toml --all-features -- -D warnings

test-main:
	cargo test --all-features --workspace

test-main-expensive:
	cargo test --all-features --workspace -- --ignored

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

build-main:
	cargo build --workspace

build-contracts:
	cargo build --manifest-path contracts/Cargo.toml --workspace

build-wallet:
	cargo build --manifest-path nym-wallet/Cargo.toml --workspace

build-connect:
	cargo build --manifest-path nym-connect/Cargo.toml --workspace

fmt-main:
	cargo fmt --all

fmt-contracts:
	cargo fmt --manifest-path contracts/Cargo.toml --all

fmt-wallet:
	cargo fmt --manifest-path nym-wallet/Cargo.toml --all

fmt-connect:
	cargo fmt --manifest-path nym-connect/Cargo.toml --all

wasm:
	RUSTFLAGS='-C link-arg=-s' cargo build --manifest-path contracts/Cargo.toml --release --target wasm32-unknown-unknown

generate-typescript:
	cd tools/ts-rs-cli && cargo run && cd ../..
	yarn types:lint:fix
