ifndef $(GOROOT)
    GOROOT=$(shell go env GOROOT)
    export GOROOT
endif

build-debug-dev:
	GOOS=js GOARCH=wasm go build -o build/go_conn.wasm ./cmd/wasm
	cp "$(GOROOT)/misc/wasm/wasm_exec.js" build

build-go:
	GOOS=js GOARCH=wasm go build -o ../../../dist/wasm/mix-fetch/go_conn.wasm ./cmd/wasm
	cp "$(GOROOT)/misc/wasm/wasm_exec.js" ../../../dist/wasm/mix-fetch


build-go-opt:
# use the -s and -w linker flags to strip the debugging information
	GOOS=js GOARCH=wasm go build -ldflags="-s -w" -o ../../../dist/wasm/mix-fetch/go_conn.wasm ./cmd/wasm
	cp "$(GOROOT)/misc/wasm/wasm_exec.js" ../../../dist/wasm/mix-fetch

# run wasm-opt to gain few more kb
	wasm-opt --enable-bulk-memory -Oz ../../../dist/wasm/mix-fetch/go_conn.wasm -o ../../../dist/wasm/mix-fetch/go_conn.wasm
# finally compress it (... or not?)

check-fmt:
	test -z $(gofmt -l .)