all: build


build:
	@mkdir -p build/
	@go build -mod=mod -o build/clayd ./cmd/clayd
	@go build -mod=mod -o build/claycli ./cmd/claycli
