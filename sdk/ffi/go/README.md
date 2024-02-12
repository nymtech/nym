# Go FFI
> ⚠️ This is an initial version of this library in order to give developers something to experiment with. If you use this code to begin testing out Mixnet integration and run into issues, errors, or have feedback, please feel free to open an issue; feedback from developers trying to use it will help us improve it. If you have questions feel free to reach out via our [Matrix channel](https://matrix.to/#/#dev:nymtech.chat).

This repo contains: 
* `lib.rs`: an initial version of bindings for interacting with the Mixnet via the Rust SDK from Go. These are essentially match statemtns wrapping imported functions from the `nym-ffi-shared` lib. 
* `ffi/`: a directory containing: 
  * the `bindings/` files generated using [`uniffi-bindgen-go`]()
  * `main.go`: an example of using this library. 

The example `main.go` file is a simple example flow of:
* setting up Nym client logging
* creating an ephemeral Nym client (no key storage / persistent address - this will come in a future iteration)
* getting its [Nym address](https://nymtech.net/docs/clients/addressing-system.html)
* using that address to send a message to yourself via the Mixnet
* listen for and parse the incoming message for the `sender_tag` used for [anonymous replies with SURBs](https://nymtech.net/docs/architecture/traffic-flow.html#private-replies-using-surbs)
* send a reply to yourself using SURBs

## Useage 
The `build.sh` script in the root of the repository speeds up the task of building and linking the Rust and Go code.
* if want to quickly recompile your code run it as-is with `./build.sh`
* if you want to clean build both the Rust and Go code after removing existing compiled binaries run it with the optional `clean` argument: `./build.sh clean`.

> Make sure to run the script from the root of the project directory.

This script will:
* (optionally if called with `clean` argument) remove existing Rust and Go artifacts
* build `lib.rs` with the `--release` flag
* compile the Go bindings 

**WIP** you need to manually add the following `cgo` flags to the generated bindings (working on automating this):
```
// #cgo LDFLAGS: -L../../target/release -lnym_go_ffi
```





```
# install uniffi-bindgen-go - make sure to use the same version as uniffirs that is in Cargo.toml
cargo install ... 

# if unset run from projroot 
# this will b set in a script as well when not WIP 
RUST_BINARIES=target/release
echo 'export LD_LIBRARY_PATH=${LD_LIBRARY_PATH}:'${RUST_BINARIES} >> ~/.zshrc
source ~/.zshrc

# (optional) clean everything 
./clean.sh

# build everything 
./build.sh 

# manually add the following line under #include math.h` in compiled Go: 
# this is a temporary hack that will b hidden in build script
// #cgo LDFLAGS: -L../../target/release -lnym_go_ffi

# run test go file 
go run ffi/main.go
```
 
