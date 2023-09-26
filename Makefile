# Top-level Makefile for the nym monorepo

# Default target. Probably what you want to run in normal day-to-day usage when
# you want to check all backend code in one step.
all: test

help:
	@echo "The main targets are"
	@echo "  - build: all binaries and tests"
	@echo "  - test: same as default target"
	@echo "  - clippy: run clippy for all workspaces"
	@echo "  - build: build all workspaces"
	@echo "  - build-release: build platform binaries and contracts in release mode"

# -----------------------------------------------------------------------------
# Meta targets
# -----------------------------------------------------------------------------

# Run clippy for all workspaces, run all tests, format all Rust code
test: clippy cargo-test fmt

# Same as test, but also runs slow tests
test-all: test cargo-test-expensive

# List `test`, but builds instead of running clippy
no-clippy: build cargo-test fmt

# Build release binaries for the main workspace (platform binaries) and the
# contracts, including running wasm-opt.
# Producing release versions of other components is deferred to their
# respective toolchains.
build-release: build-release-main contracts

# Not a meta target, more of a top-level target for building all binaries (in
# debug mode). Listed here for visibility. The deps are appended successively
build:

# Not a meta target, more of a top-level target for clippy. Listed here for
# visibility. The deps are appended successively.
clippy:

# Deprecated? Since it includes test is also includes non-happy clippy...
happy: fmt clippy-happy test

# -----------------------------------------------------------------------------
# Define targets for a given workspace
#  $(1): name
#  $(2): path to workspace
#  $(3): extra arguments to cargo
#  $(4): RUSTFLAGS prefix env
# -----------------------------------------------------------------------------
define add_cargo_workspace

clippy-happy-$(1):
	cargo $$($(1)_CLIPPY_TOOLCHAIN) clippy --manifest-path $(2)/Cargo.toml $(3)

clippy-$(1):
	cargo $$($(1)_CLIPPY_TOOLCHAIN) clippy --manifest-path $(2)/Cargo.toml --workspace $(3) -- -D warnings

clippy-extra-$(1):
	cargo $$($(1)_CLIPPY_TOOLCHAIN) clippy --manifest-path $(2)/Cargo.toml --workspace --examples --tests -- -D warnings

check-$(1):
	cargo check --manifest-path $(2)/Cargo.toml --workspace $(3)

test-$(1):
	cargo test --manifest-path $(2)/Cargo.toml --workspace

test-expensive-$(1):
	cargo test --manifest-path $(2)/Cargo.toml --workspace -- --ignored

build-$(1):
	cargo build --manifest-path $(2)/Cargo.toml --workspace $(3)

build-extra-$(1):
	cargo build --manifest-path $(2)/Cargo.toml --workspace --examples --tests

build-release-$(1):
	$(4) cargo $$($(1)_BUILD_RELEASE_TOOLCHAIN) build --manifest-path $(2)/Cargo.toml --workspace --release $(3)

fmt-$(1):
	cargo fmt --manifest-path $(2)/Cargo.toml --all

clippy-happy: clippy-happy-$(1)
clippy: clippy-$(1) clippy-extra-$(1)
check: check-$(1)
cargo-test: test-$(1)
cargo-test-expensive: test-expensive-$(1)
build: build-$(1) build-extra-$(1)
build-release-all: build-release-$(1)
fmt: fmt-$(1)
endef

# -----------------------------------------------------------------------------
# Rust workspaces
# -----------------------------------------------------------------------------

# Generate targets for the various cargo workspaces

$(eval $(call add_cargo_workspace,main,.))
$(eval $(call add_cargo_workspace,contracts,contracts,--lib --target wasm32-unknown-unknown,RUSTFLAGS='-C link-arg=-s'))
$(eval $(call add_cargo_workspace,wallet,nym-wallet))
$(eval $(call add_cargo_workspace,connect,nym-connect/desktop))

# OVERRIDE: there is an issue where clippy crashes on nym-wallet-types with the latest
# stable toolchain. So pin to 1.71.0 until that is resolved.
wallet_CLIPPY_TOOLCHAIN := +1.71.0

# OVERRIDE: wasm-opt fails if the binary has been built with the latest rustc.
# Pin to the last working version.
contracts_BUILD_RELEASE_TOOLCHAIN := +1.69.0

# -----------------------------------------------------------------------------
# SDK
# -----------------------------------------------------------------------------

sdk-wasm: sdk-wasm-build sdk-wasm-test sdk-wasm-lint

sdk-wasm-build:
	$(MAKE) -C nym-browser-extension/storage wasm-pack
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
WASM_CRATES = extension-storage nym-client-wasm nym-node-tester-wasm nym-wasm-sdk

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

