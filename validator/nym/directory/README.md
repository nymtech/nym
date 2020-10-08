# Nym Directory Server

A PKI, presence and mixmining monitoring server.

Nym nodes and clients use it to find each other, and bootstrap the network into existence. 

Mixmining reports are stored in a SQLite database at `~/.nym/mixmining.db`

## Dependencies

* Go 1.15 or later

## Building and running

The directory is integrated directly into the validator node and will start when the node starts.

## Usage

The server exposes an HTTP interface which can be queried. To see documentation 
of the server's capabilities, go to http://<deployment-host>:8081/swagger/index.html in
your browser once you've run the server. You'll be presented with an overview
of functionality. All methods are runnable through the Swagger docs interface, 
so you can poke at the server to see what it does. 

## Developing

`go test ./...` will run the test suite.

From the top-level `validator` directory, `swag init -g directory/server.go --output directory/docs/` rebuilds the Swagger docs.

If you update any of the HTML assets,
`go-assets-builder server/html/index.html -o server/html/index.go` will
put it in the correct place to be built into the binary. 

