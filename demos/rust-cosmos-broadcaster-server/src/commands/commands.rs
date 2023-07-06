/* 
code for sequence and chain 
    // possibly remote client that doesn't do ANY signing
    // (only broadcasts + queries for sequence numbers)
    let broadcaster = HttpClient::new(validator).unwrap();

    // get signer information
    let sequence_response = broadcaster.get_sequence(&signer_address).await.unwrap();
    let chain_id = broadcaster.get_chain_id().await.unwrap();
    -> pass back chain_id and sequence_response to client side 

code for broadcast 
    // decode the base58 tx to vec<u8>

    // broadcast the tx
    let res = rpc::Client::broadcast_tx_commit(&broadcaster, tx_bytes.into())
    .await
    .unwrap();

    // send res back via SURBs 
 */
