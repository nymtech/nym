use multi_tcp_client::{Client, Config};
use std::time;
use std::time::Duration;
use tokio::prelude::*;
use tokio::runtime::Runtime;

fn main() {
    let mut rt = Runtime::new().unwrap();
    let addr = "127.0.0.1:5000".parse().unwrap();
    let reconnection_backoff = Duration::from_secs(1);

    let client_config = Config::new(vec![addr], reconnection_backoff, 10 * reconnection_backoff);

    let mut c = rt.block_on(Client::new(client_config));

    for _ in 0..50 {
        rt.block_on(c.send(addr, b"foomp\n"));
        rt.block_on(async move { tokio::time::delay_for(time::Duration::from_millis(250)).await });
    }
}
