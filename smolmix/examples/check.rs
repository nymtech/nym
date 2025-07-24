use reqwest;
use std::{
    io::{Read, Write},
    net::TcpStream,
    process::Command,
};

// These are just for checking:
// - Using the proper IP info in the other tests
// - That network connection is fine otherwise outside of the `NymIprDevice`
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing connectivity to httpbin.org...\n");

    println!("DNS Resolution:");
    let output = Command::new("nslookup")
        .arg("httpbin.org")
        .output()
        .expect("Failed to execute nslookup");
    println!("{}", String::from_utf8_lossy(&output.stdout));

    println!("\nHTTP GET request:");
    match reqwest::get("http://httpbin.org/get").await {
        Ok(response) => {
            println!("Status: {}", response.status());
            println!("Headers: {:#?}", response.headers());
            let body = response.text().await?;
            println!("Body:\n{}", body);
        }
        Err(e) => {
            println!("Request failed: {}", e);
        }
    }

    println!("\nRaw TCP connection test:");

    match TcpStream::connect("httpbin.org:80") {
        Ok(mut stream) => {
            println!("Connected to httpbin.org:80");

            let request = b"GET /get HTTP/1.1\r\n\
                          Host: httpbin.org\r\n\
                          User-Agent: test/1.0\r\n\
                          Accept: */*\r\n\
                          Connection: close\r\n\
                          \r\n";

            stream.write_all(request)?;
            println!("Sent request");

            let mut response = Vec::new();
            stream.read_to_end(&mut response)?;
            println!(
                "Response ({} bytes):\n{}",
                response.len(),
                String::from_utf8_lossy(&response[..500.min(response.len())])
            );
        }
        Err(e) => {
            println!("Connection failed: {}", e);
        }
    }

    Ok(())
}
