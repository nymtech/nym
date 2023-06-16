ifndef $(GOROOT)
    GOROOT=$(shell go env GOROOT)
    export GOROOT
endif

clean:
	rm internal-static/main.wasm 2> /dev/null

build:
	GOOS=js GOARCH=wasm go build -o internal-static/main.wasm cmd/wasm/main.go && cp "$(GOROOT)/misc/wasm/wasm_exec.js" ./internal-static

run-internal:
	go run ./cmd/webserver/main.go