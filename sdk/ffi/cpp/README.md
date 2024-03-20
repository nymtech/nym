# C++ FFI 
> ⚠️ This is an initial version of this library in order to give developers something to experiment with. If you use this code to begin testing out Mixnet integration and run into issues, errors, or have feedback, please feel free to open an issue; feedback from developers trying to use it will help us improve it. If you have questions feel free to reach out via our [Matrix channel](https://matrix.to/#/#dev:nymtech.chat).  

This repo contains:
* `lib.rs`: an initial version of bindings for interacting with the Mixnet via the Rust SDK from C++. These are essentially match statements wrapping imported functions from the `nym-ffi-shared` lib allowing for nicer [error handling](#error-handling-).  
* `main.cpp`: an example of using this library, relying on `Boost` for threads. 

The example `.cpp` file is a simple example flow of: 
* setting up Nym client logging 
* creating an ephemeral Nym client (no key storage / persistent address - this will come in a future iteration)
* getting its [Nym address](https://nymtech.net/docs/clients/addressing-system.html)
* using that address to send a message to yourself via the Mixnet 
* listen for and parse the incoming message for the `sender_tag` used for [anonymous replies with SURBs](https://nymtech.net/docs/architecture/traffic-flow.html#private-replies-using-surbs)
* send a reply to yourself using SURBs

## Installation 
Prerequisites: 
* Rust
* C++  
* [Boost](https://www.boost.org/) which can be installed with:
```
# Arch / Manjaro 
yay -S boost boost-libs 

# Debian / Ubuntu 
sudo apt install libboost-all-dev
```

## Usage
The `build.sh` script in the root of the repository speeds up the task of building and linking the Rust and C++ code. 
* if want to quickly recompile your code run it as-is with `./build.sh` 
* if you want to clean build both the Rust and C++ code after removing existing compiled binaries run it with the optional `clean` argument: `./build.sh clean`. 
 
> Make sure to run the script from the root of the project directory. 

This script will: 
* (optionally if called with `clean` argument) remove existing Rust and C++ artifacts
* build `lib.rs` with the `--release` flag
* compile `main.cpp`, linking `lib.rs` 
* set value of `LD_LIBRARY_PATH` to the Rust code in `target/release/`
* run the compiled `main`

## Error Handling 
When calling a function across the FFI boundary (e.g.) `reply`, the Rust code is matching the output of an `_internal` function - `Res` or `Err` - to a member of the `StatusCode` enum. This allows for both Rust-style error handling and the ease of returning a `c_int` across the FFI boundary, which can be used by C++ for its own error handling / conditional logic.


