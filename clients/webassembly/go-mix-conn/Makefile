ifndef $(GOROOT)
    GOROOT=$(shell go env GOROOT)
    export GOROOT
endif


ifndef $(TINYGOROOT)
    TINYGOROOT=$(shell tinygo env TINYGOROOT)
    export TINYGOROOT
endif

build-go:
	#tinygo build -o internal-static/main.wasm -target wasm cmd/wasm/main.go # && cp "$(GOROOT)/misc/wasm/wasm_exec.js" ./internal-static
	GOOS=js GOARCH=wasm go build -o build/main.wasm ./cmd/wasm && cp "$(GOROOT)/misc/wasm/wasm_exec.js" ./build

build-tinygo:
# the default stack size of 16kib is insufficient
	tinygo build -o build/main.wasm -stack-size "32kb" --no-debug ./cmd/wasm && cp "$(TINYGOROOT)/targets/wasm_exec.js" ./build

