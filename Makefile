# Default target
all: test

test: clippy-all cargo-test wasm fmt

test-all: test cargo-test-expensive

no-clippy: build cargo-test wasm fmt

happy: fmt clippy-happy test

# -----------------------------------------------------------------------------
# Define targets for a given workspace
#  $(1): name
#  $(2): path to workspace
#  $(3): extra arguments to cargo
# -----------------------------------------------------------------------------
define add_cargo_workspace

clippy-happy-$(1):
	cargo clippy --manifest-path $(2)/Cargo.toml $(3)

clippy-$(1):
	cargo clippy --manifest-path $(2)/Cargo.toml --workspace $(3) -- -D warnings

clippy-$(1)-examples:
	cargo clippy --manifest-path $(2)/Cargo.toml --workspace --examples -- -D warnings

check-$(1):
	cargo check --manifest-path $(2)/Cargo.toml --workspace $(3)

test-$(1):
	cargo test --manifest-path $(2)/Cargo.toml --workspace

test-$(1)-expensive:
	cargo test --manifest-path $(2)/Cargo.toml --workspace -- --ignored

build-$(1):
	cargo build --manifest-path $(2)/Cargo.toml --workspace $(3)

build-$(1)-examples:
	cargo build --manifest-path $(2)/Cargo.toml --workspace --examples

fmt-$(1):
	cargo fmt --manifest-path $(2)/Cargo.toml --all

clippy-happy: clippy-happy-$(1)
clippy-all: clippy-$(1) clippy-$(1)-examples
check: check-$(1)
cargo-test: test-$(1)
cargo-test-expensive: test-$(1)-expensive
build: build-$(1) build-$(1)-examples
fmt: fmt-$(1)

endef

# -----------------------------------------------------------------------------
# Rust workspaces
# -----------------------------------------------------------------------------

# Generate targets for the various cargo workspaces

$(eval $(call add_cargo_workspace,main,.))
$(eval $(call add_cargo_workspace,contracts,contracts,--target wasm32-unknown-unknown))
$(eval $(call add_cargo_workspace,wasm-client,clients/webassembly,--target wasm32-unknown-unknown))
$(eval $(call add_cargo_workspace,wallet,nym-wallet,))
$(eval $(call add_cargo_workspace,connect,nym-connect/desktop))
ifdef NYM_MOBILE
$(eval $(call add_cargo_workspace,connect-mobile,nym-connect/mobile/src-tauri))
endif

# -----------------------------------------------------------------------------
# Convenience targets for crates that are already part of the main workspace
# -----------------------------------------------------------------------------

build-explorer-api:
	cargo build -p explorer-api

build-nym-cli:
	cargo build -p nym-cli --release

# -----------------------------------------------------------------------------
# Misc
# -----------------------------------------------------------------------------

wasm:
	RUSTFLAGS='-C link-arg=-s' cargo build --manifest-path contracts/Cargo.toml --release --target wasm32-unknown-unknown
	wasm-opt --disable-sign-ext -Os contracts/target/wasm32-unknown-unknown/release/vesting_contract.wasm -o contracts/target/wasm32-unknown-unknown/release/vesting_contract.wasm
	wasm-opt --disable-sign-ext -Os contracts/target/wasm32-unknown-unknown/release/mixnet_contract.wasm -o contracts/target/wasm32-unknown-unknown/release/mixnet_contract.wasm
	wasm-opt --disable-sign-ext -Os contracts/target/wasm32-unknown-unknown/release/nym_service_provider_directory.wasm -o contracts/target/wasm32-unknown-unknown/release/nym_service_provider_directory.wasm

# NOTE: this seems deprecated an not needed anymore?
mixnet-opt: wasm
	cd contracts/mixnet && make opt

generate-typescript:
	cd tools/ts-rs-cli && cargo run && cd ../..
	yarn types:lint:fix

run-api-tests:
	cd nym-api/tests/functional_test && yarn test:qa
