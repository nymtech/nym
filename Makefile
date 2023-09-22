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

# -----------------------------------------------------------------------------
# Meta targets
# -----------------------------------------------------------------------------

test: clippy cargo-test fmt

test-all: test cargo-test-expensive

no-clippy: build cargo-test fmt

# Deprecated? Since it includes test is also includes non-happy clippy...
happy: fmt clippy-happy test

# Building release binaries is a little manual as we can't just build --release
# on all workspaces. Cherry-pick these two.
build-release: build-release-main contracts

# Not a meta target, more of a top-level target for building all binaries (in
# debug mode). Listed here for visibility. The deps are appended successively
build:

# Not a meta target, more of a top-level target for clippy. Listed here for
# visibility. The deps are appended successively.
clippy:

# -----------------------------------------------------------------------------
# Define targets for a given workspace
#  $(1): name
#  $(2): path to workspace
#  $(3): extra arguments to cargo
#  $(4): prefix env
# -----------------------------------------------------------------------------
define add_cargo_workspace

clippy-happy-$(1):
	$(4) cargo clippy --manifest-path $(2)/Cargo.toml $(3)

clippy-$(1):
	$(4) cargo clippy --manifest-path $(2)/Cargo.toml --workspace $(3) -- -D warnings

clippy-examples-$(1):
	$(4) cargo clippy --manifest-path $(2)/Cargo.toml --workspace --examples -- -D warnings

check-$(1):
	$(4) cargo check --manifest-path $(2)/Cargo.toml --workspace $(3)

test-$(1):
	$(4) cargo test --manifest-path $(2)/Cargo.toml --workspace

test-expensive-$(1):
	$(4) cargo test --manifest-path $(2)/Cargo.toml --workspace -- --ignored

build-standalone-$(1):
	$(4) cargo build --manifest-path $(2)/Cargo.toml $(3)

build-$(1):
	$(4) cargo build --manifest-path $(2)/Cargo.toml --workspace $(3)

build-examples-$(1):
	$(4) cargo build --manifest-path $(2)/Cargo.toml --workspace --examples

build-release-$(1):
	$(4) cargo build --manifest-path $(2)/Cargo.toml --workspace --release $(3)

fmt-$(1):
	$(4) cargo fmt --manifest-path $(2)/Cargo.toml --all

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
$(eval $(call add_cargo_workspace,contracts,contracts,--lib --target wasm32-unknown-unknown,RUSTFLAGS='-C link-arg=-s'))
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

# Add to top-level targets
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

# NOTE: These targets are part of the main workspace (but not as wasm32-unknown-unknown)
WASM_CRATES = nym-client-wasm nym-node-tester-wasm nym-wasm-sdk

sdk-wasm-test:
	#cargo test $(addprefix -p , $(WASM_CRATES)) --target wasm32-unknown-unknown -- -Dwarnings

sdk-wasm-lint:
	cargo clippy $(addprefix -p , $(WASM_CRATES)) --target wasm32-unknown-unknown -- -Dwarnings
	$(MAKE) -C wasm/mix-fetch check-fmt

# Add to top-level targets
build: sdk-wasm-build
cargo-test: sdk-wasm-test
clippy: sdk-wasm-lint

# -----------------------------------------------------------------------------
# Build contracts ready for deploy
# -----------------------------------------------------------------------------

CONTRACTS=vesting_contract mixnet_contract nym_service_provider_directory nym_name_service
CONTRACTS_WASM=$(addsuffix .wasm, $(CONTRACTS))
CONTRACTS_OUT_DIR=contracts/target/wasm32-unknown-unknown/release

contracts: build-release-contracts wasm-opt-contracts

wasm-opt-contracts:
	for contract in $(CONTRACTS_WASM); do \
	  wasm-opt --disable-sign-ext -Os $(CONTRACTS_OUT_DIR)/$$contract -o $(CONTRACTS_OUT_DIR)/$$contract; \
	done

# Consider adding 's' to make plural consistent (beware: used in github workflow)
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

