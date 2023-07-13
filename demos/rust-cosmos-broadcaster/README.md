TODO FOR DEMO
- clippy
- pass mnemonic as file 
- nicer logging - show sequence response etc
- pass tx hash back to client nicely & remove from service 

### Nym mixnet cosmos tx broadcaster demo 

A demo showing how to: 
* sign a cosmos tx (simple token transfer) offline 
* broadcast this tx from a service on the other side of the mixnet

For the moment the fact its a token transfer is hardcoded, but this was just due to time constraints. I plan to continue building this out into a multi-functional wrapper allowing for queries, custom txs, wasm contract interaction, etc. 

Built using: 
* rust sdk 
* validator client libs (that will soon be part of the sdk)

#### Useage 
```
# compile
cargo build --release

example 1: sign & send @ same time 
# start service
../../target/release/service

# sign tx - when prompted enter 'y' 
../../target/release/client offline-sign-tx "<MNEMONIC>" "<RECIPIENT_NYX_ADDRESS>

example 2: sign first, send later 
# start service 
# sign tx - when prompted enter 'n' and copy encoded tx bytes from terminal 
# send tx using encoded bytes as arg 
```
