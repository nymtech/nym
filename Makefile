# Top-level Makefile for the nym monorepo
#
# Common targets:
#
#   - `make` build all binaries and tests
#   - `make test`: same as default target
#   - `make clippy`: run clippy for all workspaces
#   - `make build`: build all workspaces
#   - `make build-release`: build binaries in release mode
#

# Default target
all: test

test: clippy cargo-test contracts-wasm sdk-wasm-test fmt

test-all: test cargo-test-expensive

no-clippy: build cargo-test contracts-wasm fmt

# Deprecated? Since it includes test is also includes non-happy clippy...
happy: fmt clippy-happy test

# Meta target for building all binaries (in debug mode)
build:

# Building release binaries is a little manual as we can't just build --release
# on all workspaces.
build-release: build-release-main contracts-wasm

# Meta target for clippy
clippy:

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

build-standalone-$(1):
	cargo build --manifest-path $(2)/Cargo.toml $(3)

build-$(1):
	cargo build --manifest-path $(2)/Cargo.toml --workspace $(3)

build-examples-$(1):
	cargo build --manifest-path $(2)/Cargo.toml --workspace --examples

build-release-$(1):
	cargo build --manifest-path $(2)/Cargo.toml --workspace --release $(3)

fmt-$(1):
	cargo fmt --manifest-path $(2)/Cargo.toml --all

clippy-happy: clippy-happy-$(1)
clippy: clippy-$(1) clippy-examples-$(1)
check: check-$(1)
cargo-test: test-$(1)
cargo-test-expensive: test-expensive-$(1)
build: build-$(1) build-examples-$(1)
build-release-all: build-release-$(1)
fmt: fmt-$(1)

endef

# -----------------------------------------------------------------------------
# Rust workspaces
# -----------------------------------------------------------------------------

# Generate targets for the various cargo workspaces

$(eval $(call add_cargo_workspace,main,.))
$(eval $(call add_cargo_workspace,contracts,contracts,--lib --target wasm32-unknown-unknown))
$(eval $(call add_cargo_workspace,wallet,nym-wallet,))
$(eval $(call add_cargo_workspace,connect,nym-connect/desktop))

# -----------------------------------------------------------------------------
# Browser extension
# -----------------------------------------------------------------------------

# Binary is part of main workspace, but not as wasm32-unknown-unknown
# NOTE: do we need this? I'd imagine wasm-pack is the one actually used
build-browser-extension-storage:
	cargo build -p extension-storage --target wasm32-unknown-unknown

wasm-pack-browser-extension-storage:
	$(MAKE) -C nym-browser-extension/storage wasm-pack

# Target is part of main workspace, but not as wasm32-unknown-unknown
clippy-browser-extension-storage:
	cargo clippy -p extension-storage --target wasm32-unknown-unknown -- -Dwarnings

# Add to meta targets
build: build-browser-extension-storage
clippy: clippy-browser-extension-storage

# -----------------------------------------------------------------------------
# SDK
# -----------------------------------------------------------------------------

sdk-wasm: sdk-wasm-build sdk-wasm-test sdk-wasm-lint

# NOTE: think about this dependency, is it needed?
sdk-wasm-build: wasm-pack-browser-extension-storage
	$(MAKE) -C wasm/client
	$(MAKE) -C wasm/node-tester
	$(MAKE) -C wasm/mix-fetch
	$(MAKE) -C wasm/full-nym-wasm

# run this from npm/yarn to ensure tools are in the path, e.g. yarn build:sdk from root of repo
sdk-typescript-build:
	npx lerna run --scope @nymproject/sdk build --stream
	npx lerna run --scope @nymproject/mix-fetch build --stream
	npx lerna run --scope @nymproject/node-tester build --stream
	yarn --cwd sdk/typescript/codegen/contract-clients build

sdk-wasm-test:
#	# client
#	cargo test -p nym-client-wasm --target wasm32-unknown-unknown
#
#	# node-tester
#	cargo test -p nym-node-tester-wasm --target wasm32-unknown-unknown
#
#	# mix-fetch
#	#cargo test -p nym-wasm-sdk --target wasm32-unknown-unknown
#
#	# full
#	cargo test -p nym-wasm-sdk --target wasm32-unknown-unknown


# NOTE: These targets are part of the main workspace (but not as wasm32-unknown-unknown)
WASM_CRATES = nym-client-wasm nym-node-tester-wasm nym-wasm-sdk

sdk-wasm-lint:
	cargo clippy $(addprefix -p , $(WASM_CRATES)) --target wasm32-unknown-unknown -- -Dwarnings
	$(MAKE) -C wasm/mix-fetch check-fmt

# Add to meta targets
build: sdk-wasm-build
clippy: sdk-wasm-lint

# -----------------------------------------------------------------------------
# Contracts
# -----------------------------------------------------------------------------

CONTRACTS_OUT_DIR=contracts/target/wasm32-unknown-unknown/release
VESTING_CONTRACT=$(CONTRACTS_OUT_DIR)/vesting_contract.wasm
MIXNET_CONTRACT=$(CONTRACTS_OUT_DIR)/mixnet_contract.wasm
SERVICE_PROVIDER_DIRECTORY_CONTRACT=$(CONTRACTS_OUT_DIR)/nym_service_provider_directory.wasm
NAME_SERVICE_CONTRACT=$(CONTRACTS_OUT_DIR)/nym_name_service.wasm

contracts-wasm: contracts-wasm-build contracts-wasm-opt

contracts-wasm-build:
	RUSTFLAGS='-C link-arg=-s' cargo build --lib --manifest-path contracts/Cargo.toml --release --target wasm32-unknown-unknown

contracts-wasm-opt:
	wasm-opt --disable-sign-ext -Os $(VESTING_CONTRACT) -o $(VESTING_CONTRACT)
	wasm-opt --disable-sign-ext -Os $(MIXNET_CONTRACT) -o $(MIXNET_CONTRACT)
	wasm-opt --disable-sign-ext -Os $(SERVICE_PROVIDER_DIRECTORY_CONTRACT) -o $(SERVICE_PROVIDER_DIRECTORY_CONTRACT)
	wasm-opt --disable-sign-ext -Os $(NAME_SERVICE_CONTRACT) -o $(NAME_SERVICE_CONTRACT)

contract-schema:
	$(MAKE) -C contracts schema

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

generate-typescript:
	cd tools/ts-rs-cli && cargo run && cd ../..
	yarn types:lint:fix

run-api-tests:
	cd nym-api/tests/functional_test && yarn test:qa

