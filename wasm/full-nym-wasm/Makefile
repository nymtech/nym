UNAME_S := $(shell uname -s)
ifeq ($(UNAME_S),Linux)
	TASKSET := taskset -c 0-11
else
	TASKSET :=
endif

all: build-full

build-full:
	$(TASKSET) wasm-pack build --all-features --scope nymproject --target web --out-dir ../../dist/wasm/full-nym-wasm
	$(TASKSET) wasm-opt -Oz -o ../../dist/wasm/full-nym-wasm/nym_wasm_sdk_bg.wasm ../../dist/wasm/full-nym-wasm/nym_wasm_sdk_bg.wasm
