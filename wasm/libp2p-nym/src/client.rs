// Copyright 2024 Nym Technologies SA
// SPDX-License-Identifier: Apache-2.0

//! Client initialization for the libp2p transport.
//!
//! This module provides a simple way to create a Nym client configured
//! for use with the libp2p transport.

use nym_client_wasm::client::NymClientBuilder;
use nym_client_wasm::config::{ClientConfig, ClientConfigOpts};
use nym_client_wasm::stream::NymClientStream;
use nym_sphinx_addressing::clients::Recipient;
use nym_wasm_utils::console_log;
use std::str::FromStr;

use crate::error::Error;

// Type alias to help with inference
type WasmClientError = nym_client_wasm::error::WasmClientError;

/// Result of creating a transport client.
pub struct TransportClient {
    /// Our Nym address
    pub self_address: Recipient,
    /// The client stream for sending/receiving messages
    pub stream: NymClientStream,
}

/// Options for creating a transport client.
#[derive(Default)]
pub struct TransportClientOpts {
    /// Optional Nym API URL override
    pub nym_api_url: Option<String>,
    /// Force TLS connections to gateways (required for browser environments)
    pub force_tls: bool,
    /// Client ID for storage namespace (avoids conflicts with other clients)
    pub client_id: Option<String>,
}

/// Create a Nym client configured for libp2p transport use.
///
/// This connects to the Nym network and returns a `NymClientStream`
/// that implements `Stream` for receiving messages.
///
/// # Example
/// ```ignore
/// use nym_libp2p_wasm::{create_transport_client_async, TransportClientOpts, NymTransport};
/// use libp2p_identity::Keypair;
/// use futures::StreamExt;
///
/// // Create the transport client (connects to Nym network)
/// let opts = TransportClientOpts { force_tls: true, ..Default::default() };
/// let result = create_transport_client_async(opts).await?;
///
/// // Use the stream directly
/// while let Some(msg) = result.stream.next().await {
///     println!("Received: {:?}", msg);
/// }
/// ```
pub async fn create_transport_client_async(
    opts: TransportClientOpts,
) -> Result<TransportClient, Error> {
    let client_id = opts
        .client_id
        .unwrap_or_else(|| "libp2p-transport".to_string());
    console_log!(
        "Creating transport client (id={}, force_tls={})...",
        client_id,
        opts.force_tls
    );

    // Create config with client ID for storage namespace isolation
    let config_opts = ClientConfigOpts {
        id: Some(client_id),
        nym_api: opts.nym_api_url,
        nyxd: None,
        debug: None,
    };
    let config: ClientConfig = ClientConfig::new(config_opts)
        .map_err(|e: WasmClientError| Error::ClientCreationFailed(e.to_string()))?;

    // Create builder with a dummy handler (we won't use JS callbacks)
    let dummy_handler = js_sys::Function::new_no_args("");
    let builder = NymClientBuilder::new(config, dummy_handler, opts.force_tls, None, None);

    // Start client for transport use - returns (NymClient, ClientOutput)
    let (client, client_output) = builder
        .start_client_for_transport()
        .await
        .map_err(|e: WasmClientError| Error::ClientCreationFailed(e.to_string()))?;

    let address_str = client.self_address();
    let self_address = Recipient::from_str(&address_str).map_err(Error::InvalidRecipientBytes)?;

    console_log!("Transport client ready at: {}", address_str);

    // Wrap in NymClientStream for easy async usage
    let stream = NymClientStream::new(client, client_output);

    Ok(TransportClient {
        self_address,
        stream,
    })
}
