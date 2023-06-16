ifndef $(GOROOT)
    GOROOT=$(shell go env GOROOT)
    export GOROOT
endif

clean:
	rm internal-static/main.wasm 2> /dev/null

build:
	#tinygo build -o internal-static/main.wasm -target wasm cmd/wasm/main.go # && cp "$(GOROOT)/misc/wasm/wasm_exec.js" ./internal-static
	GOOS=js GOARCH=wasm go build -o internal-static/main.wasm cmd/wasm/main.go && cp "$(GOROOT)/misc/wasm/wasm_exec.js" ./internal-static
	cp internal-static/main.wasm pkg/main.wasm
	cp internal-static/wasm_exec.js pkg/wasm_exec.js


run-internal:
	go run ./cmd/webserver/main.go