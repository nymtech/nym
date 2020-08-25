// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// The only reason this exists is to remove duplicate code from
// nym\service-providers\simple-socks5\src\connection.rs::try_read_response_data
// and
// nym\clients\socks5\src\socks\request.rs::try_read_request_data

// once those use sequence numbers, this code should be removed!!

use crate::available_reader::AvailableReader;
use std::io;
use tokio::io::AsyncRead;
use tokio::time::Duration;

// It returns data alognside information whether it timed out while reading from the socket
pub async fn try_read_data<R>(
    timeout: Duration,
    mut reader: R,
    address: &str,
) -> io::Result<(Vec<u8>, bool)>
where
    R: AsyncRead + Unpin,
{
    let mut data = Vec::new();
    let mut delay = tokio::time::delay_for(timeout);

    let mut available_reader = AvailableReader::new(&mut reader);

    loop {
        tokio::select! {
            _ = &mut delay => {
                println!("Timed out. returning {} bytes received from {}", data.len(), address);
                return Ok((data, true)) // we return all response data on timeout
            }
            read_data = &mut available_reader => {
                match read_data {
                    Err(err) => {
                        return Err(err);
                    }
                    Ok(bytes) => {
                        if bytes.len() == 0 {
                            println!("Connection is closed! Returning {} bytes received from {}", data.len(), address);
                            // we return all we managed to read because
                            // we know no more stuff is coming
                            return Ok((data, false))
                        }
                        let now = tokio::time::Instant::now();
                        let next = now + timeout;
                        delay.reset(next);
                        println!("Received {} bytes from {}. Waiting for more...", bytes.len(), address);

                        // temporarily this is fine... (this loop will go away anyway)
                        data.extend_from_slice(&bytes)
                    }
                }
            }
        }
    }
}
