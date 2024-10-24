# Autodoc

WIP command output documentation generator. Run via `../scripts/next-scripts/autodoc.sh` to create a bunch of markdown files which are then moved around for importing into the documentation.

## `Error: Os { code: 2, kind: NotFound, message: "No such file or directory" }`
If you run into this error, make sure that you have the following directory structure:

```sh
autodoc/
├── autodoc-generated-markdown/
│     └── commands/
├── Cargo.toml
├── README.md
└── src
    └── main.rs
```

And if you don't - create it and re-run.

If you are encountering this error with this dir structure in place, check that all Nym binaries that are listed in `main.rs` also exist in `nym/target/release/`.

Run this crate on its own with `debug` logging and it should panic on the missing binary:

```sh
 RUST_LOG="debug" cargo run --release
    Finished `release` profile [optimized] target(s) in 0.20s
     Running `/home/______/src/nym/target/release/autodoc`
[2024-10-23T08:11:01Z DEBUG autodoc] now running Some(
        "nym-api",
    )
[2024-10-23T08:11:01Z DEBUG autodoc] stderr: " 2024-10-23T08:11:01.981Z INFO  nym_api > Starting nym api...\n"
[2024-10-23T08:11:01Z DEBUG autodoc] stderr: " 2024-10-23T08:11:01.985Z INFO  nym_api > Starting nym api...\n"
[2024-10-23T08:11:01Z INFO  autodoc] SKIPPING ../../target/release/nym-api init
[2024-10-23T08:11:01Z INFO  autodoc] creating own file for ../../target/release/nym-api init --help
[2024-10-23T08:11:01Z DEBUG autodoc] stderr: " 2024-10-23T08:11:01.993Z INFO  nym_api > Starting nym api...\n"

< snip >

[2024-10-23T08:11:02Z DEBUG autodoc] now running Some(
        "nym-cli",
    )
Error: Os { code: 2, kind: NotFound, message: "No such file or directory" }

```
