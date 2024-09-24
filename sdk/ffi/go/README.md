# Go FFI
> ⚠️ This is an initial version of this library in order to give developers something to experiment with. If you use this code to begin testing out Mixnet integration and run into issues, errors, or have feedback, please feel free to open an issue; feedback from developers trying to use it will help us improve it. If you have questions feel free to reach out via our [Matrix channel](https://matrix.to/#/#dev:nymtech.chat).

This repo contains:
* `lib.rs`: an initial version of bindings for interacting with the Mixnet via the Rust SDK from Go. These are essentially match statemtns wrapping imported functions from the `nym-ffi-shared` lib.
* `ffi/`: a directory containing:
  * the `bindings/` files generated using [`uniffi-bindgen-go`](https://github.com/NordSecurity/uniffi-bindgen-go)
  * [`example.go`](./example.go): an example of using the mixnet client functionality.
  * [`proxy_example.go`](./proxy_example.go): an example of using the TcpProxy functionality.

## Useage - Consuming the Library
You can import the bindings as normal and interact with them as shown in the example files. These files import the bindings from this repository (hence the `go.mod` and `go.sum` in the crate root) but you can import them remotely as usual.

## Useage - Developing on the Library
If you want to fork and add new features/functions to this library use the following instructions to rebuild the Go bindings.

Rust functions exposed to the Go binding library are in `./src/lib.rs`.

The `build.sh` script in the root of the repository speeds up the task of building and linking the Rust and Go code.
* if want to quickly recompile your code run it as-is with `./build.sh`
* if you want to clean build both the Rust and Go code after removing existing compiled binaries run it with the optional `clean` argument: `./build.sh clean`.

> Make sure to run the script from the root of the project directory, and that your LD PATH is set first!
> ```
> RUST_BINARIES=../../../target/release
> echo 'export LD_LIBRARY_PATH=${LD_LIBRARY_PATH}:'${RUST_BINARIES} >> ~/.zshrc
> source ~/.zshrc
> ```

This script will:
* (optionally if called with `clean` argument) remove existing Rust and Go artifacts
* build `lib.rs` with the `--release` flag
* compile the Go bindings

**WIP** you need to manually add the following `cgo` flags to the generated bindings immediately underneath LN3 (`// #include <bindings.h`). In the future this will be automated in `build.sh`:

```
// #cgo LDFLAGS: -L../../../../../target/release -lnym_go_ffi
```
