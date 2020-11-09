use std::time::Duration;

use tokio::time;

mod mixmining;
mod topology;

pub async fn start() {
    let mut timer = time::interval(Duration::from_secs(10));
    loop {
        timer.tick().await;

        match topology::renew_periodically().await {
            Err(err) => println!("Error refreshing topology: {}", err),
            Ok(()) => (),
        };

        match mixmining::renew_periodically().await {
            Err(err) => println!("Error refreshing mixmining report: {}", err),
            Ok(()) => (),
        };
    }
}
