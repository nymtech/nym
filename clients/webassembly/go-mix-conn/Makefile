ifndef $(GOROOT)
    GOROOT=$(shell go env GOROOT)
    export GOROOT
endif


ifndef $(TINYGOROOT)
    TINYGOROOT=$(shell tinygo env TINYGOROOT)
    export TINYGOROOT
endif

build-go:
	GOOS=js GOARCH=wasm go build -o build/main.wasm ./cmd/wasm && cp "$(GOROOT)/misc/wasm/wasm_exec.js" ./build

build-go-opt:
# use the -s and -w linker flags to strip the debugging information
	GOOS=js GOARCH=wasm go build -ldflags="-s -w" -o build/main.wasm ./cmd/wasm && cp "$(GOROOT)/misc/wasm/wasm_exec.js" ./build
# run wasm-opt to gain few more kb
	wasm-opt --enable-bulk-memory -Oz build/main.wasm -o build/main.wasm
# finally compress it (... or not?)

build-tinygo:
# the default stack size of 16kib is insufficient
# note: it seems to leak memory : (
# ref: https://github.com/tinygo-org/tinygo/issues/1140
	tinygo build -o build/main.wasm -stack-size "32kb" ./cmd/wasm && cp "$(TINYGOROOT)/targets/wasm_exec.js" ./build

