# Top-level Makefile for the nym monorepo

# Default target. Probably what you want to run in normal day-to-day usage when
# you want to check all backend code in one step.
all: test

help:
	@echo "The main targets are"
	@echo "  all: the default target. Alias for test"
	@echo "  build: build all binaries"
	@echo "  build-release: build platform binaries and contracts in release mode"
	@echo "  clippy: run clippy for all workspaces"
	@echo "  test: run clippy, unit tests, and formatting."
	@echo "  test-all: like test, but also includes the expensive tests"
	@echo "  deb: build debian packages"
	@echo ""
	@echo "Contract building targets:"
	@echo "  contracts: build contracts for development (includes wasm-opt)"
	@echo "  publish-contracts: build contracts using Docker optimizer (deterministic)"

# -----------------------------------------------------------------------------
# Meta targets
# -----------------------------------------------------------------------------

# Run clippy for all workspaces, run all tests, format all Rust code
test: clippy cargo-test fmt

# Same as test, but also runs slow tests
test-all: test cargo-test-expensive

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

# -----------------------------------------------------------------------------
# Define targets for a given workspace
#  $(1): name
#  $(2): path to workspace
#  $(3): extra arguments to cargo
#  $(4): RUSTFLAGS prefix env
# -----------------------------------------------------------------------------
define add_cargo_workspace

check-$(1):
	cargo check --manifest-path $(2)/Cargo.toml --workspace $(3)

build-$(1):
	cargo build --manifest-path $(2)/Cargo.toml --workspace $(3)

build-extra-$(1):
	cargo build --manifest-path $(2)/Cargo.toml --workspace --examples --tests

build-release-$(1):
	$(4) cargo $$($(1)_BUILD_RELEASE_TOOLCHAIN) build --manifest-path $(2)/Cargo.toml --workspace --release $(3)

test-$(1):
	cargo test --manifest-path $(2)/Cargo.toml --workspace

test-expensive-$(1):
	cargo test --manifest-path $(2)/Cargo.toml --workspace -- --ignored

clippy-$(1):
	cargo $$($(1)_CLIPPY_TOOLCHAIN) clippy --manifest-path $(2)/Cargo.toml --workspace $(3) -- -D warnings

clippy-extra-$(1):
	cargo $$($(1)_CLIPPY_TOOLCHAIN) clippy --manifest-path $(2)/Cargo.toml --workspace --examples --tests -- -D warnings

fmt-$(1):
	cargo fmt --manifest-path $(2)/Cargo.toml --all

check: check-$(1)
build: build-$(1) build-extra-$(1)
build-release-all: build-release-$(1)
cargo-test: test-$(1)
cargo-test-expensive: test-expensive-$(1)
clippy: clippy-$(1) clippy-extra-$(1)
fmt: fmt-$(1)
endef

# -----------------------------------------------------------------------------
# Rust workspaces
# -----------------------------------------------------------------------------

# Generate targets for the various cargo workspaces

$(eval $(call add_cargo_workspace,main,.))
$(eval $(call add_cargo_workspace,contracts,contracts,--lib --target wasm32-unknown-unknown,RUSTFLAGS='-C link-arg=-s'))
$(eval $(call add_cargo_workspace,wallet,nym-wallet))

# -----------------------------------------------------------------------------
# SDK
# -----------------------------------------------------------------------------

sdk-wasm: sdk-wasm-build sdk-wasm-test sdk-wasm-lint

sdk-wasm-build:
	$(MAKE) -C nym-browser-extension/storage wasm-pack
	$(MAKE) -C wasm/client
	$(MAKE) -C wasm/node-tester
	$(MAKE) -C wasm/mix-fetch
	$(MAKE) -C wasm/zknym-lib
	#$(MAKE) -C wasm/full-nym-wasm

# run this from npm/yarn to ensure tools are in the path, e.g. yarn build:sdk from root of repo
sdk-typescript-build:
	npx lerna run --scope @nymproject/sdk build --stream
	npx lerna run --scope @nymproject/mix-fetch build --stream
	npx lerna run --scope @nymproject/node-tester build --stream
	yarn --cwd sdk/typescript/codegen/contract-clients build

# NOTE: These targets are part of the main workspace (but not as wasm32-unknown-unknown)
WASM_CRATES = extension-storage nym-client-wasm nym-node-tester-wasm zknym-lib

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
# Build CosmWasm contracts (deterministic docker build)
# -----------------------------------------------------------------------------


WASM_CONTRACT_DIR := contracts/target/wasm32-unknown-unknown/release
# Find every direct contract folder that contains a Cargo.toml
CONTRACT_DIRS := $(shell find contracts -type f -name Cargo.toml \( ! -path "contracts/Cargo.toml" \) | grep -v integration-tests | xargs -n1 dirname | sort -u)

CONTRACTS_OUT_DIR = contracts/artifacts

# Build all contracts via the official CosmWasm optimizer image (one invocation per contract)
# See : https://github.com/CosmWasm/optimizer?tab=readme-ov-file#contracts-excluded-from-workspace
# The optimizer ships separate multi-arch images. ARM builds are *not* bit-for-bit identical to the
# canonical x86_64 build (see README notice in CosmWasm/optimizer).  For reproducible artefacts we
# therefore always run the amd64 variant by default.
#   Override with :
#   $ COSMWASM_OPTIMIZER_IMAGE=cosmwasm/optimizer-arm64:0.17.0 make contracts-publish
#
COSMWASM_OPTIMIZER_IMAGE ?= cosmwasm/optimizer:0.17.0
COSMWASM_OPTIMIZER_PLATFORM ?= linux/amd64

# Ensure clean build environment and run the optimizer
optimize-contracts:
	@rm -rf artifacts 2>/dev/null || true
	@echo "=== Ensuring clean build environment"
	docker volume rm nym_contracts_cache 2>/dev/null || true
	docker volume rm registry_cache 2>/dev/null || true
	@for DIR in $(CONTRACT_DIRS); do \
	  echo "=== Optimizing $${DIR}"; \
	  docker run --rm --platform $(COSMWASM_OPTIMIZER_PLATFORM) \
	    -v $(CURDIR):/code \
	    --mount type=volume,source=nym_contracts_cache,target=/target \
	    --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
	    -e CARGO_BUILD_INCREMENTAL=false \
	    -e RUSTFLAGS="-C target-cpu=generic -C debuginfo=0" \
	    -e SOURCE_DATE_EPOCH=1 \
	    $(COSMWASM_OPTIMIZER_IMAGE) $${DIR}; \
	done
	@mkdir -p $(CONTRACTS_OUT_DIR)
	@cp artifacts/*.wasm $(CONTRACTS_OUT_DIR)/ 2>/dev/null || true

	@cd $(CONTRACTS_OUT_DIR) && sha256sum *.wasm > checksums.txt
	# Cleanup temporary artefacts directory
	@rm -rf artifacts 2>/dev/null || true

wasm-opt-contracts:
	@for WASM in $(WASM_CONTRACT_DIR)/*.wasm; do \
	  echo "Running wasm-opt on $$WASM"; \
	  wasm-opt --signext-lowering -Os $$WASM -o $$WASM ; \
	done

cosmwasm-check-contracts:
	@for WASM in $(WASM_CONTRACT_DIR)/*.wasm; do \
	  echo "Checking $$WASM"; \
	  cosmwasm-check $$WASM ; \
	done

# Default development build
contracts: build-release-contracts wasm-opt-contracts

# Publishing build used by CI – deterministic Docker optimiser
publish-contracts: optimize-contracts

# Consider adding 's' to make plural consistent (beware: used in github workflow)
contract-schema:
	$(MAKE) -C contracts schema

# -----------------------------------------------------------------------------
# Convenience targets for crates that are already part of the main workspace
# -----------------------------------------------------------------------------

build-nym-cli:
	cargo build -p nym-cli --release

# -----------------------------------------------------------------------------
# Misc
# -----------------------------------------------------------------------------

generate-typescript:
	cd tools/ts-rs-cli && cargo run && cd ../..
	yarn types:lint:fix

# Run the integration tests for public nym-api endpoints
run-api-tests:
	dotenv -f envs/sandbox.env -- cargo test --test public-api-tests

# Build debian package, and update PPA
deb-cli: build-nym-cli
	cargo deb -p nym-cli

deb: deb-cli
