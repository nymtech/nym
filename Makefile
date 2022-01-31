all: clippy-all test fmt wasm
happy: clippy-happy test fmt
clippy-all: clippy-all-main clippy-all-contracts clippy-all-wallet
clippy-happy: clippy-happy-main clippy-happy-contracts clippy-happy-wallet
test: test-main test-contracts test-wallet
fmt: fmt-main fmt-contracts fmt-wallet

clippy-happy-main:
	cargo clippy

clippy-happy-contracts:
	cargo clippy --manifest-path contracts/Cargo.toml --target wasm32-unknown-unknown

clippy-happy-wallet: 
	cargo clippy --manifest-path nym-wallet/Cargo.toml

clippy-all-main:
	cargo clippy --all-features -- -D warnings 

clippy-all-contracts:
	cargo clippy --manifest-path contracts/Cargo.toml --all-features --target wasm32-unknown-unknown -- -D warnings

clippy-all-wallet: 
	cargo clippy --manifest-path nym-wallet/Cargo.toml --all-features -- -D warnings

test-main:
	cargo test --all-features

test-contracts:
	cargo test --manifest-path contracts/Cargo.toml --all-features

test-wallet:
	cargo test --manifest-path nym-wallet/Cargo.toml --all-features

fmt-main:
	cargo fmt --all

fmt-contracts:
	cargo fmt --manifest-path contracts/Cargo.toml --all

fmt-wallet:
	cargo fmt --manifest-path nym-wallet/Cargo.toml --all

wasm:
	RUSTFLAGS='-C link-arg=-s' cargo build --manifest-path contracts/Cargo.toml --release --target wasm32-unknown-unknown
