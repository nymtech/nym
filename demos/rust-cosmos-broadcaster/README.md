### Nym mixnet cosmos tx broadcaster demo 
A demo showing how to: 
* sign a cosmos tx (simple token transfer) offline 
* broadcast this tx from a service on the other side of the mixnet

For the moment the fact its a token transfer is hardcoded. This code could be built out to allow for queries, custom txs, wasm contract interaction, etc but goes beyond the bounds of this demo. 

Built using: 
* rust sdk 
* validator client libs (that will soon be part of the sdk)

#### Useage 
```
# compile
cargo build --release

example 1: sign & send in one go 
# start service
../../target/release/service

# copy service's nym address to use as value of <SERVICE_NYM_ADDRESS> 

# sign tx - when prompted enter 'y' 
../../target/release/client offline-sign-tx ${SENDER_MNEMONIC} <RECIPIENT_NYX_ADDRESS> <SERVICE_NYM_ADDRESS>

example 2: create signed tx 
# start service 
../../target/release/service

# sign tx - when prompted enter 'n' and copy encoded tx bytes from terminal 
../../target/release/client offline-sign-tx ${SENDER_MNEMONIC} <RECIPIENT_NYX_ADDRESS> <SERVICE_NYM_ADDRESS>

# send tx using encoded bytes as arg 
../../target/release/client send-tx <COPIED_BYTES> <SERVICE_NYM_ADDRESS>
```
