# Preparing Your Environment

## Prerequisites
* `Rust` & `cargo`

## Creating your Project Structure

* Make a new cargo project:
```
cargo new nym-cosmos-service
```

* Create the following directory structure and files:
```
.
├── Cargo.toml
├── bin
│   ├── client.rs
│   └── service.rs
└── src
    ├── client.rs
    ├── lib.rs
    ├── main.rs
    └── service.rs

3 directories, 7 files
```

* Finally add the following dependencies - you can just copy and paste this into your `Cargo.toml` file:
```
TODO pull validator client code etc from the monorepo instead of workspace loading
```
