FROM rust as rust-builder
RUN git clone https://github.com/nymtech/nym.git
RUN rustup target add wasm32-unknown-unknown
WORKDIR nym/contracts/mixnet
# Checkout to an older version, until the new one is stabilized
RUN git checkout f08f19cd86fa045982898318bb59f539a67c8eaa
RUN RUSTFLAGS='-C link-arg=-s' cargo build --release --target wasm32-unknown-unknown

FROM node
COPY upload-wasm.ts .
COPY package.json .
COPY --from=rust-builder /nym/clients/validator/ /validator/
COPY --from=rust-builder /nym/contracts/mixnet/target/wasm32-unknown-unknown/release/mixnet_contracts.wasm .
RUN npm install
WORKDIR /validator
RUN yarn install
RUN yarn build
WORKDIR /
RUN yarn install
WORKDIR /
COPY init_and_start.sh .
ENTRYPOINT ["./init_and_start.sh"]
