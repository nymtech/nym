all: clippy test wasm fmt
clippy: clippy-main clippy-contracts clippy-wallet
test: test-main test-contracts test-wallet
fmt: fmt-main fmt-contracts fmt-wallet

clippy-main:
	cargo clippy

clippy-contracts:
	cargo clippy --manifest-path contracts/Cargo.toml

clippy-wallet: 
	cargo clippy --manifest-path nym-wallet/Cargo.toml

test-main:
	cargo test

test-contracts:
	cargo test --manifest-path contracts/Cargo.toml

test-wallet:
	cargo test --manifest-path nym-wallet/Cargo.toml

fmt-main:
	cargo fmt --all

fmt-contracts:
	cargo fmt --manifest-path contracts/Cargo.toml --all

fmt-wallet:
	cargo fmt --manifest-path nym-wallet/Cargo.toml --all

wasm:
	RUSTFLAGS='-C link-arg=-s' cargo build --manifest-path contracts/Cargo.toml --release --target wasm32-unknown-unknown
