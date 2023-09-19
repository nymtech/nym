ifndef $(GOROOT)
    GOROOT=$(shell go env GOROOT)
    export GOROOT
endif


ifndef $(TINYGOROOT)
    TINYGOROOT=$(shell tinygo env TINYGOROOT)
    export TINYGOROOT
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

build-tinygo:
# the default stack size of 16kib is insufficient
# note: it seems to leak memory : (
# ref: https://github.com/tinygo-org/tinygo/issues/1140
# HOWEVER. it could be severely reduced via: https://github.com/tinygo-org/tinygo/issues/1140#issuecomment-718145455
	tinygo build -o ../../../dist/wasm/mix-fetch/go_conn.wasm -stack-size "32kb" ./cmd/wasm && cp "$(TINYGOROOT)/targets/wasm_exec.js" ../../../dist/wasm/mix-fetch

check-fmt:
	test -z $(gofmt -l .)