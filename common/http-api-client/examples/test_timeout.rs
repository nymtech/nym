use nym_http_api_client::registry;
use nym_http_api_client::{ReqwestClientBuilder, inventory};
use nym_http_api_client_macro::client_defaults;
use std::time::{Duration, Instant};

#[tokio::main]
async fn main() {
    println!("Testing HTTP Client Timeout Configuration");
    println!("==========================================\n");

    client_defaults!(timeout = std::time::Duration::from_secs(300),);

    // Build a client using the registry (should have 300s timeout)
    let client = registry::build_client().expect("Failed to build client");

    println!("Testing timeout behavior...");
    println!("The inventory should have set timeout to 300 seconds");

    // Test 1: Try a request to a slow endpoint that delays for 5 seconds
    // This should succeed since timeout is 300s
    println!("\nTest 1: Request with 5 second delay (should succeed)");
    let start = Instant::now();
    match client.get("https://httpbin.org/delay/5").send().await {
        Ok(_) => {
            let elapsed = start.elapsed();
            println!("✓ Request succeeded after {:?}", elapsed);
        }
        Err(e) => {
            let elapsed = start.elapsed();
            if e.is_timeout() {
                println!(
                    "✗ Request timed out after {:?} - timeout might be shorter than expected!",
                    elapsed
                );
            } else {
                println!("✗ Request failed after {:?}: {}", elapsed, e);
            }
        }
    }

    // Test 2: Try to inspect the client's actual configuration
    println!("\nTest 2: Client debug information");
    println!("Client Debug: {:?}", client);

    // Test 3: Create a client with explicit short timeout to compare behavior
    println!("\nTest 3: Control test with 2 second timeout");
    let short_timeout_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .expect("Failed to build short timeout client");

    let start = Instant::now();
    match short_timeout_client
        .get("https://httpbin.org/delay/5")
        .send()
        .await
    {
        Ok(_) => {
            let elapsed = start.elapsed();
            println!(
                "✗ Request succeeded after {:?} - timeout not working!",
                elapsed
            );
        }
        Err(e) => {
            let elapsed = start.elapsed();
            if e.is_timeout() {
                println!("✓ Request timed out as expected after {:?}", elapsed);
            } else {
                println!("? Request failed after {:?}: {}", elapsed, e);
            }
        }
    }

    // Test 4: Create a client through the registry and verify timeout on a hanging connection
    println!("\nTest 4: Testing with a connection that hangs");
    println!("Making request to an endpoint that will hang...");

    let start = Instant::now();
    // This IP is reserved for documentation and will hang
    match client.get("http://192.0.2.1:81").send().await {
        Ok(_) => {
            println!("✗ Request succeeded - unexpected!");
        }
        Err(e) => {
            let elapsed = start.elapsed();
            if e.is_timeout() {
                println!("✓ Request timed out after {:?}", elapsed);
                if elapsed < Duration::from_secs(290) {
                    println!(
                        "  Note: Timeout occurred faster than 300s, might be connection timeout not total timeout"
                    );
                }
            } else if e.is_connect() {
                println!("✓ Connection failed after {:?} (connect timeout)", elapsed);
            } else {
                println!("? Request failed after {:?}: {}", elapsed, e);
            }
        }
    }
}
