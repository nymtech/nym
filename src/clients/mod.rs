use sphinx::route::DestinationAddressBytes;
use tokio::runtime::Runtime;

pub mod directory;
pub mod mix;
pub mod provider;
pub mod validator;

struct Client {
    // to be replaced by something else I guess
    address: DestinationAddressBytes
}

type TripleFutureResult = (Result<(), Box<dyn std::error::Error>>, Result<(), Box<dyn std::error::Error>>, Result<(), Box<dyn std::error::Error>>);

impl Client {
    pub fn new(address: DestinationAddressBytes) -> Self {
        Client {
            address
        }
    }

    async fn start_loop_cover_traffic_stream(&self) -> Result<(), Box<dyn std::error::Error>> {
        unimplemented!()
    }

    async fn control_out_queue(&self) -> Result<(), Box<dyn std::error::Error>> {
        unimplemented!()
    }


    async fn start_provider_polling(&self) -> Result<(), Box<dyn std::error::Error>> {
        unimplemented!()
    }


    async fn start_traffic(&self) -> TripleFutureResult {
        futures::future::join3(self.start_loop_cover_traffic_stream(), self.control_out_queue(), self.start_provider_polling()).await
    }

    pub fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut rt = Runtime::new()?;

        rt.block_on(async {
            let future_results = self.start_traffic().await;
            assert!(future_results.0.is_ok() && future_results.1.is_ok() && future_results.2.is_ok());
        });

        // this line in theory should never be reached as the runtime should be permanently blocked on traffic senders
        eprintln!("The client went kaput...");
        Ok(())
    }
}