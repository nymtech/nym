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
example 1: sign & send @ same time 
# start service 
# sign tx - when prompted enter 'y' 

example 2: sign first, send later 
# start service 
# sign tx - when prompted enter 'n' and copy encoded tx bytes from terminal 
# send tx using encoded bytes as arg 
```


//

- bin 
    - client  : `main.rs` from `rust...client/` 
    - service : `main.rs` from `rust...server/` 

- src 
    - lib     : `reqres` definitions + define CONSTs in there 
    - client  : `commands.rs` from `rust...client/`   
    - service : `commands.rs` from `rust...server/` 

- added CONSTS to
    - client src DONE 
    - client bin -- make sp an optional cli arg otherwise set as DEFAULT
    - service src 
    - service bin 