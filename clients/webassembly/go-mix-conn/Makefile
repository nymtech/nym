ifndef $(GOROOT)
    GOROOT=$(shell go env GOROOT)
    export GOROOT
endif


ifndef $(TINYGOROOT)
    TINYGOROOT=$(shell tinygo env TINYGOROOT)
    export TINYGOROOT
endif

build:
	#tinygo build -o internal-static/main.wasm -target wasm cmd/wasm/main.go # && cp "$(GOROOT)/misc/wasm/wasm_exec.js" ./internal-static
	GOOS=js GOARCH=wasm go build -o pkg/main.wasm ./cmd/wasm && cp "$(GOROOT)/misc/wasm/wasm_exec.js" ./pkg

build-tinygo:
# the default stack size of 16kib is insufficient
	tinygo build -o pkg/main.wasm -stack-size "32kb" --no-debug ./cmd/wasm && cp "$(TINYGOROOT)/targets/wasm_exec.js" ./pkg

