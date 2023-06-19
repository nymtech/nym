ifndef $(GOROOT)
    GOROOT=$(shell go env GOROOT)
    export GOROOT
endif

build:
	#tinygo build -o internal-static/main.wasm -target wasm cmd/wasm/main.go # && cp "$(GOROOT)/misc/wasm/wasm_exec.js" ./internal-static
	GOOS=js GOARCH=wasm go build -o pkg/main.wasm cmd/wasm/main.go cmd/wasm/logger.go cmd/wasm/fake_connection.go cmd/wasm/ssl_helper.go && cp "$(GOROOT)/misc/wasm/wasm_exec.js" ./pkg
