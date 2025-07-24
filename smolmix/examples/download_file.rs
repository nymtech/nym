use nym_sdk::stream_wrapper::IpMixStream;
use smolmix::create_device;
use smoltcp::{
    iface::{Config, Interface, SocketSet},
    socket::tcp,
    time::Instant,
    wire::{HardwareAddress, IpAddress, IpCidr, Ipv4Address},
};
use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
    time::Duration,
};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let download_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Downloads")
        .join("nym-test");

    fs::create_dir_all(&download_dir)?;
    let file_path = download_dir.join("1Mb.dat");
    info!("Will save to: {}", file_path.display());

    info!("Connecting to Mixnet...");
    let ipr_stream = IpMixStream::new().await?;
    let (mut device, bridge, allocated_ips) = create_device(ipr_stream).await?;

    info!("Allocated IP: {}", allocated_ips.ipv4);

    // Bridge has to be run in its own task as per its docs
    tokio::spawn(async move {
        if let Err(e) = bridge.run().await {
            eprintln!("Bridge error: {}", e);
        }
    });

    // Create smoltcp interface using our allocated IP
    let config = Config::new(HardwareAddress::Ip);
    let mut iface = Interface::new(config, &mut device, Instant::now());

    // Configure with our allocated IP TODO can probably smush this + fn above into one / somewhere else so doesn't have to be done manually
    iface.update_ip_addrs(|ip_addrs| {
        ip_addrs
            .push(IpCidr::new(IpAddress::from(allocated_ips.ipv4), 32))
            .unwrap();
    });

    // Add default route
    iface
        .routes_mut()
        .add_default_ipv4_route(Ipv4Address::UNSPECIFIED)
        .unwrap();

    let tcp_rx_buffer = tcp::SocketBuffer::new(vec![0; 65536]);
    let tcp_tx_buffer = tcp::SocketBuffer::new(vec![0; 4096]);
    let tcp_socket = tcp::Socket::new(tcp_rx_buffer, tcp_tx_buffer);

    let mut sockets = SocketSet::new(vec![]);
    let tcp_handle = sockets.add(tcp_socket);

    let target_ip = Ipv4Address::new(3, 213, 24, 5); // httpbin.org IP - if you're getting a mangled response run check.rs and make sure you're using the correct IP
    let remote_path = "/bytes/10240"; // 10KB
    let target_port = 80;
    let expected_size = 10240;

    info!("Downloading {}:{}{}", target_ip, target_port, remote_path);

    let mut file = File::create(&file_path)?;
    let mut connected = false;
    let mut request_sent = false;
    let mut total_bytes = 0;
    let mut body_bytes = 0;
    let mut headers_complete = false;
    let mut header_buffer = Vec::new();
    let mut timestamp = Instant::from_millis(0);
    let start = tokio::time::Instant::now();

    while start.elapsed() < Duration::from_secs(300) {
        iface.poll(timestamp, &mut device, &mut sockets);

        let socket = sockets.get_mut::<tcp::Socket>(tcp_handle);

        if !connected && !socket.is_open() {
            // iface is the hook into the NymIpDevice
            socket.connect(iface.context(), (target_ip, target_port), 49152)?;
            info!("Connecting...");
            connected = true;
        }

        if socket.state() == tcp::State::Established && !request_sent {
            let http_request = format!(
                "GET {} HTTP/1.1\r\n\
                 Host: httpbin.org\r\n\
                 User-Agent: smolmix/0.1\r\n\
                 Connection: close\r\n\
                 \r\n",
                remote_path
            );

            socket.send_slice(http_request.as_bytes())?;
            info!("Request sent");
            request_sent = true;
        }

        if socket.can_recv() {
            socket.recv(|buffer| {
                let len = buffer.len();
                total_bytes += len;

                if !headers_complete {
                    header_buffer.extend_from_slice(buffer);

                    // Check for end of headers
                    if let Ok(headers_str) = std::str::from_utf8(&header_buffer) {
                        if let Some(header_end) = headers_str.find("\r\n\r\n") {
                            headers_complete = true;

                            // Write body portion to file
                            let body_start = header_end + 4;
                            if body_start < header_buffer.len() {
                                let body_chunk = &header_buffer[body_start..];
                                file.write_all(body_chunk).unwrap();
                                body_bytes = body_chunk.len();
                                info!("Headers complete, writing body to file");
                            }
                        }
                    }
                } else {
                    // Headers done, write body directly to file
                    file.write_all(buffer).unwrap();
                    body_bytes += len;
                }

                // Progress update
                if body_bytes > 0 && body_bytes % 10240 < len {
                    let progress = (body_bytes as f64 / expected_size as f64) * 100.0;
                    let elapsed = start.elapsed().as_secs_f64();
                    let throughput = (body_bytes as f64 / elapsed) / 1024.0;

                    info!(
                        "Progress: {:.1}% ({} / {} bytes) - {:.1} KB/s",
                        progress, body_bytes, expected_size, throughput
                    );
                }

                (len, ())
            })?;
        }

        // Check completion
        if body_bytes >= expected_size || (socket.state() == tcp::State::Closed && body_bytes > 0) {
            break;
        }

        timestamp += smoltcp::time::Duration::from_millis(10);
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Ensure file is flushed
    file.flush()?;

    // Final report
    let elapsed = start.elapsed().as_secs_f64();
    let throughput = (body_bytes as f64 / elapsed) / 1024.0;

    info!("\nDownload Complete!");
    info!("Saved to: {}", file_path.display());
    info!("File size: {} bytes", body_bytes);
    info!("Total time: {:.1} seconds", elapsed);
    info!("Average speed: {:.1} KB/s", throughput);

    // Verify file
    let metadata = fs::metadata(&file_path)?;
    info!("File on disk: {} bytes", metadata.len());
    info!("Success: {}", metadata.len() as usize == expected_size);

    // TODO make a graceful shutdown fn for Device -> IprWrapper

    Ok(())
}
