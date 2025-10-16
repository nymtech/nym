build-bypass-contract:
	$(MAKE) -C dkg-bypass-contract build


COSMWASM_OPTIMIZER_IMAGE ?= cosmwasm/optimizer:0.17.0
COSMWASM_OPTIMIZER_PLATFORM ?= linux/amd64

build-bypass-contract-docker:
	docker volume rm nym_contracts_cache 2>/dev/null || true
	docker volume rm registry_cache 2>/dev/null || true
	docker run --rm --platform $(COSMWASM_OPTIMIZER_PLATFORM) \
		-v $(CURDIR)/../../..:/code \
		--mount type=volume,source=nym_contracts_cache,target=/target \
		--mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
		-e CARGO_BUILD_INCREMENTAL=false \
		-e RUSTFLAGS="-C target-cpu=generic -C debuginfo=0" \
		-e SOURCE_DATE_EPOCH=1 \
		$(COSMWASM_OPTIMIZER_IMAGE) "tools/internal/testnet-manager/dkg-bypass-contract"; \
