# Default target
all: test

test: clippy-all cargo-test contracts-wasm sdk-wasm-test fmt

test-all: test cargo-test-expensive

no-clippy: build cargo-test contracts-wasm fmt

happy: fmt clippy-happy test

build: sdk-wasm-build

# Building release binaries is a little manual as we can't just build --release
# on all workspaces.
build-release: build-release-main contracts-wasm

clippy: sdk-wasm-lint

# Deprecated
# For backwards compatibility
clippy-all: clippy

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
$(eval $(call add_cargo_workspace,contracts,contracts,--target wasm32-unknown-unknown))
#$(eval $(call add_cargo_workspace,wasm-client,clients/webassembly,--target wasm32-unknown-unknown))
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

sdk-wasm: sdk-wasm-build sdk-wasm-test sdk-wasm-lint

sdk-wasm-build:
	# client
	cargo build -p nym-client-wasm --target wasm32-unknown-unknown

	# node-tester
	cargo build -p nym-node-tester-wasm --target wasm32-unknown-unknown

	# mix-fetch
	$(MAKE) -C wasm/mix-fetch build

	# full
	cargo build -p nym-wasm-sdk --target wasm32-unknown-unknown

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


sdk-wasm-lint:
	# client
	cargo clippy -p nym-client-wasm --target wasm32-unknown-unknown -- -Dwarnings

	# node-tester
	cargo clippy -p nym-node-tester-wasm --target wasm32-unknown-unknown -- -Dwarnings

	# mix-fetch
	$(MAKE) -C wasm/mix-fetch check-fmt

	# full
	cargo clippy -p nym-wasm-sdk --target wasm32-unknown-unknown -- -Dwarnings


# -----------------------------------------------------------------------------
# Build contracts ready for deploy
# -----------------------------------------------------------------------------

CONTRACTS_OUT_DIR=contracts/target/wasm32-unknown-unknown/release
VESTING_CONTRACT=$(CONTRACTS_OUT_DIR)/vesting_contract.wasm
MIXNET_CONTRACT=$(CONTRACTS_OUT_DIR)/mixnet_contract.wasm
SERVICE_PROVIDER_DIRECTORY_CONTRACT=$(CONTRACTS_OUT_DIR)/nym_service_provider_directory.wasm
NAME_SERVICE_CONTRACT=$(CONTRACTS_OUT_DIR)/nym_name_service.wasm

contracts-wasm: contracts-wasm-build contracts-wasm-opt

contracts-wasm-build:
	RUSTFLAGS='-C link-arg=-s' cargo build --manifest-path contracts/Cargo.toml --release --target wasm32-unknown-unknown

contracts-wasm-opt:
	wasm-opt --disable-sign-ext -Os $(VESTING_CONTRACT) -o $(VESTING_CONTRACT)
	wasm-opt --disable-sign-ext -Os $(MIXNET_CONTRACT) -o $(MIXNET_CONTRACT)
	wasm-opt --disable-sign-ext -Os $(SERVICE_PROVIDER_DIRECTORY_CONTRACT) -o $(SERVICE_PROVIDER_DIRECTORY_CONTRACT)
	wasm-opt --disable-sign-ext -Os $(NAME_SERVICE_CONTRACT) -o $(NAME_SERVICE_CONTRACT)

# -----------------------------------------------------------------------------
# Misc
# -----------------------------------------------------------------------------

# NOTE: this seems deprecated an not needed anymore?
mixnet-opt: contracts-wasm
	cd contracts/mixnet && make opt

generate-typescript:
	cd tools/ts-rs-cli && cargo run && cd ../..
	yarn types:lint:fix

run-api-tests:
	cd nym-api/tests/functional_test && yarn test:qa
