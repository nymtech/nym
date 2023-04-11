# Default target
all: test

test: clippy-all cargo-test wasm fmt

test-all: test cargo-test-expensive

no-clippy: build cargo-test wasm fmt

happy: fmt clippy-happy test

# Building release binaries is a little manual as we can't just build --release
# on all workspaces.
build-release: build-release-main wasm

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

clippy-examples-$(1):
	cargo clippy --manifest-path $(2)/Cargo.toml --workspace --examples -- -D warnings

check-$(1):
	cargo check --manifest-path $(2)/Cargo.toml --workspace $(3)

test-$(1):
	cargo test --manifest-path $(2)/Cargo.toml --workspace

test-expensive-$(1):
	cargo test --manifest-path $(2)/Cargo.toml --workspace -- --ignored

build-$(1):
	cargo build --manifest-path $(2)/Cargo.toml --workspace $(3)

build-examples-$(1):
	cargo build --manifest-path $(2)/Cargo.toml --workspace --examples

build-release-$(1):
	cargo build --manifest-path $(2)/Cargo.toml --workspace --release $(3)

fmt-$(1):
	cargo fmt --manifest-path $(2)/Cargo.toml --all

clippy-happy: clippy-happy-$(1)
clippy-all: clippy-$(1) clippy-examples-$(1)
check: check-$(1)
cargo-test: test-$(1)
cargo-test-expensive: test-expensive-$(1)
build: build-$(1) build-$(1)-examples
build-release-all: build-release-$(1)
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
# Build contracts ready for deploy
# -----------------------------------------------------------------------------

CONTRACTS_OUT_DIR=contracts/target/wasm32-unknown-unknown/release
VESTING_CONTRACT=$(CONTRACTS_OUT_DIR)/vesting_contract.wasm
MIXNET_CONTRACT=$(CONTRACTS_OUT_DIR)/mixnet_contract.wasm
SERVICE_PROVIDER_DIRECTORY_CONTRACT=$(CONTRACTS_OUT_DIR)/nym_service_provider_directory.wasm

wasm: wasm-build wasm-opt

wasm-build:
	RUSTFLAGS='-C link-arg=-s' cargo build --manifest-path contracts/Cargo.toml --release --target wasm32-unknown-unknown

wasm-opt:
	wasm-opt --disable-sign-ext -Os $(VESTING_CONTRACT) -o $(VESTING_CONTRACT)
	wasm-opt --disable-sign-ext -Os $(MIXNET_CONTRACT) -o $(MIXNET_CONTRACT)
	wasm-opt --disable-sign-ext -Os $(SERVICE_PROVIDER_DIRECTORY_CONTRACT) -o $(SERVICE_PROVIDER_DIRECTORY_CONTRACT)

# -----------------------------------------------------------------------------
# Misc
# -----------------------------------------------------------------------------

# NOTE: this seems deprecated an not needed anymore?
mixnet-opt: wasm
	cd contracts/mixnet && make opt

generate-typescript:
	cd tools/ts-rs-cli && cargo run && cd ../..
	yarn types:lint:fix

run-api-tests:
	cd nym-api/tests/functional_test && yarn test:qa
