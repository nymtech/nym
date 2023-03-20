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
define generate_targets_workspace

clippy-happy-$(1):
	cargo clippy --manifest-path $(2)/Cargo.toml $(3)

clippy-$(1):
	cargo clippy --manifest-path $(2)/Cargo.toml --workspace $(3) -- -D warnings

clippy-$(1)-examples:
	cargo clippy --manifest-path $(2)/Cargo.toml --workspace --examples -- -D warnings

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
cargo-test: test-$(1)
cargo-test-expensive: test-$(1)-expensive
build: build-$(1) build-$(1)-examples
fmt: fmt-$(1)
endef

# -----------------------------------------------------------------------------
# Main workspace
# -----------------------------------------------------------------------------

$(eval $(call generate_targets_workspace,main,.))

# -----------------------------------------------------------------------------
# Contracts
# -----------------------------------------------------------------------------

$(eval $(call generate_targets_workspace,contracts,contracts,--target wasm32-unknown-unknown))

# -----------------------------------------------------------------------------
# nym-wallet
# -----------------------------------------------------------------------------

$(eval $(call generate_targets_workspace,wallet,nym-wallet,))

# -----------------------------------------------------------------------------
# nym-connect
# -----------------------------------------------------------------------------

$(eval $(call generate_targets_workspace,connect,nym-connect/desktop))

ifndef NYM_NO_MOBILE
$(eval $(call generate_targets_workspace,connect-mobile,nym-connect/mobile/src-tauri))
endif

# -----------------------------------------------------------------------------
# nym-client-wasm
# -----------------------------------------------------------------------------

$(eval $(call generate_targets_workspace,wasm-client,clients/webassembly,--target wasm32-unknown-unknown))

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
	wasm-opt -Os contracts/target/wasm32-unknown-unknown/release/vesting_contract.wasm -o contracts/target/wasm32-unknown-unknown/release/vesting_contract.wasm
	wasm-opt -Os contracts/target/wasm32-unknown-unknown/release/mixnet_contract.wasm -o contracts/target/wasm32-unknown-unknown/release/mixnet_contract.wasm

mixnet-opt: wasm
	cd contracts/mixnet && make opt

generate-typescript:
	cd tools/ts-rs-cli && cargo run && cd ../..
	yarn types:lint:fix

run-api-tests:
	cd nym-api/tests/functional_test && yarn test:qa
