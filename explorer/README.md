The Nym Explorer
================

Displays nodes and metrics for the Nym network. Will eventually include a UI for viewing mixnodes, mixing rates, entropy levels, and a block explorer. 


Development
-----------

The code uses [Rocket](https://rocket.rs), which requires Rust nightly for the moment. 

You can override `rustup` on a per-directory basis from the `explorer` directory by doing `rustup override set nightly`. 

Then just `cargo run` like normal, no `+nightly` stuff needed.

