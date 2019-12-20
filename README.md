# Nym Mixnode

A Rust mixnode implementation.

## Building

* check out the code
* [install rust](https://www.rust-lang.org/tools/install) (stable)
* `cargo build --release` (for a production build)

The built binary can be found at `target/release/nym-mixnode`

## Usage

* `nym-mixnode` prints a help message showing usage options
* `nym-mixnode run --help` prints a help message showing usage options for the run command
* `nym-mixnode run --layer 1` will start the mixnode in layer 1 (coordinate with other people to find out which layer you need to start yours in)

By default, the Nym Mixnode will start on port 1789. If desired, you can change the port using the `--port` option.
