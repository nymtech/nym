use std::{
    fs::File,
    io::{BufRead as _, BufReader},
};

use bytes::BytesMut;
use log::info;
use nym_kcp::{
    codec::KcpCodec,
    packet::{KcpCommand, KcpPacket},
};
use tokio_util::codec::{Decoder as _, Encoder as _};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1) Open a file and read lines
    let file = File::open("bin/wire_format/packets.txt")?;
    let reader = BufReader::new(file);

    // 2) Create our KcpCodec
    let mut codec = KcpCodec {};

    // We'll use out_buf for encoded data from *all* lines
    let mut out_buf = BytesMut::new();

    let mut input_lines = vec![];

    // Read lines & encode them all
    for (i, line) in reader.lines().enumerate() {
        let line = line?;
        info!("Original line #{}: {}", i + 1, line);

        // Construct a KcpPacket
        let pkt = KcpPacket::new(
            42,
            KcpCommand::Push,
            0,
            128,
            0,
            i as u32,
            0,
            line.as_bytes().to_vec(),
        );

        input_lines.push(pkt.clone_data());

        // Encode (serialize) the packet into out_buf
        codec.encode(pkt, &mut out_buf)?;
    }

    // === Simulate encryption & transmission ===
    // In reality, you might do `encrypt(&out_buf)` and then
    // send it over the network. We'll just clone here:
    let mut received_buf = out_buf.clone();

    // 3) Now decode (deserialize) all packets at once
    // For demonstration, read them back out
    let mut count = 0;

    let mut decoded_lines = vec![];

    #[allow(clippy::while_let_loop)]
    loop {
        match codec.decode(&mut received_buf)? {
            Some(decoded_pkt) => {
                count += 1;
                // Convert packet data back to a string
                let decoded_str = String::from_utf8_lossy(decoded_pkt.data());
                info!("Decoded line #{}: {}", decoded_pkt.sn() + 1, decoded_str);

                decoded_lines.push(decoded_pkt.clone_data());
            }
            None => break,
        }
    }

    for (i, j) in input_lines.iter().zip(decoded_lines.iter()) {
        assert_eq!(i, j);
    }

    info!("Decoded {} lines total.", count);

    Ok(())
}
