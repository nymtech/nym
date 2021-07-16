FROM rust
RUN rustup target add wasm32-unknown-unknown
CMD cd nym/contracts/mixnet && RUSTFLAGS='-C link-arg=-s' cargo build --release --target wasm32-unknown-unknown

