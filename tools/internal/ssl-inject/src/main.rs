// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use std::io;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::select;
use tokio::time::sleep;

#[derive(Parser)]
struct Args {
    // ipv4 nymtech.net address
    #[clap(long, default_value = "185.19.28.43:443")]
    target: String,
}

fn decode_hex(raw: &str) -> Vec<u8> {
    hex::decode(raw).unwrap()
}

fn encode_hex(bytes: &[u8]) -> String {
    hex::encode(bytes)
}

fn read_line() -> anyhow::Result<String> {
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer)?;
    Ok(buffer)
}

async fn read_from_conn(conn: &mut TcpStream) -> Vec<u8> {
    let timeout = sleep(Duration::from_secs(1));
    tokio::pin!(timeout);

    let mut read = Vec::new();
    loop {
        let mut buf = [0u8; 1024];
        select! {
            _ = &mut timeout => {
                return read
            }
            res = conn.read(&mut buf) => {
                let n = res.unwrap();
                if n == 0 {
                    return read
                }
                println!("read {n} bytes");
                read.append(&mut buf[..n].to_vec())
            }

        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let target = args.target;

    println!("connecting to {target}");
    let mut conn = TcpStream::connect(target).await?;
    println!("connected");

    loop {
        println!("[Client] >>> ");
        let client_hello = read_line()?;
        let raw = decode_hex(client_hello.trim());
        conn.write_all(&raw).await?;

        let data = read_from_conn(&mut conn).await;
        let encoded = encode_hex(&data);
        println!("[Server] >>>\n{encoded}");
    }

    // println!("[ClientHello] >>> ");
    // let client_hello = read_line()?;
    // let raw = decode_hex(client_hello.trim());
    // conn.write_all(&raw).await?;
    //
    // let data = read_from_conn(&mut conn).await;
    // let encoded = encode_hex(&data);
    // println!("[ServerHello] >>>\n{encoded}");
    //
    // println!("[ClientKeyExchange] >>> ");
    // let client_response = read_line()?;
    // let raw = decode_hex(client_response.trim());
    // conn.write_all(&raw).await?;
    //
    // let data = read_from_conn(&mut conn).await;
    // let encoded = encode_hex(&data);
    // println!("[ServerFinished] >>>\n{encoded}");

    // Ok(())
}
