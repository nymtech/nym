use crate::connection::Connection;
use crate::foo::ControllerCommand;
use crate::{foo, websocket};
use futures::channel::mpsc;
use futures::stream::SplitSink;
use futures::SinkExt;
use futures_util::StreamExt;
use nymsphinx::addressing::clients::Recipient;
use simple_socks5_requests::{Request, Response};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::WebSocketStream;
use websocket::WebsocketConnectionError;
use websocket_requests::{requests::ClientRequest, responses::ServerResponse};

pub struct ServiceProvider {}

impl ServiceProvider {
    pub fn new() -> ServiceProvider {
        ServiceProvider {}
    }

    async fn mixnet_response_listener(
        mut websocket_writer: SplitSink<WebSocketStream<TcpStream>, Message>,
        mut mix_reader: mpsc::UnboundedReceiver<(Response, Recipient)>,
    ) {
        // TODO: wire SURBs in here once they're available
        while let Some((response, return_address)) = mix_reader.next().await {
            // make 'request' to native-websocket client
            let response_message = ClientRequest::Send {
                recipient: return_address,
                message: response.into_bytes(),
                with_reply_surb: false,
            };

            let message = Message::Binary(response_message.serialize());
            websocket_writer.send(message).await.unwrap();
        }
    }

    async fn run_proxy() {}

    /// Start all subsystems
    pub async fn run(&mut self) {
        let websocket_stream = self.connect_websocket("ws://localhost:1977").await;
        let (websocket_writer, mut websocket_reader) = websocket_stream.split();

        let (mix_input_sender, mut mix_input_receiver) = mpsc::unbounded::<(Response, Recipient)>();

        let (mut active_connections_controller, controller_sender) = foo::Controller::new();

        tokio::spawn(async move {
            active_connections_controller.run().await;
        });

        tokio::spawn(async move {
            Self::mixnet_response_listener(websocket_writer, mix_input_receiver).await;
        });

        while let Some(msg) = websocket_reader.next().await {
            let data = msg.unwrap().into_data();
            let received = match ServerResponse::deserialize(&data).expect("todo: error handling") {
                ServerResponse::Received(received) => received,
                ServerResponse::Error(err) => {
                    panic!("received error from native client! - {}", err)
                }
                _ => unimplemented!("probably should never be reached?"),
            };

            let raw_message = received.message;
            // TODO: here be potential SURB

            let request = Request::try_from_bytes(&raw_message).expect("todo: error handling");
            match request {
                Request::Connect {
                    conn_id,
                    remote_addr,
                    data,
                    return_address,
                } => {
                    // setup for receiving from the mixnet
                    let (mix_sender, mix_receiver) = mpsc::unbounded();
                    controller_sender
                        .unbounded_send(ControllerCommand::Insert(conn_id, mix_sender))
                        .unwrap();
                    let mix_input_sender_clone = mix_input_sender.clone();
                    tokio::spawn(async move {
                        // TODO: we must be careful to ensure we have no memory leaks here. hopefully we won't...
                        let mut conn = Connection::new(conn_id, remote_addr, &data, return_address)
                            .await
                            .expect("TODO: ERROR HANDLING");

                        conn.run_proxy(mix_receiver, mix_input_sender_clone).await;
                    });
                }
                Request::Send(conn_id, data, closed) => controller_sender
                    .unbounded_send(ControllerCommand::Send(conn_id, data, closed))
                    .unwrap(),
                Request::Close(conn_id) => {
                    // TODO: we must somehow ensure this will also kill the connection task
                    // if for some reason it's still alive
                    controller_sender
                        .unbounded_send(ControllerCommand::Remove(conn_id))
                        .unwrap();
                }
            }

            //
            //     let mut controller_local_pointer = controller.clone();
            //     let response_sender_clone = sender.clone();
            //
            //     // response_sender_clone is ALMOST our mix_sender
            //     // TODO: spawn a task with some channels?
            //
            //     // process_connect()
            //     // run_proxy()
            //     // something close
            //
            //     tokio::spawn(async move {
            //         if let Ok(response_option) = controller_local_pointer.process_request(request).await
            //         {
            //             if let Some((response, return_address)) = response_option {
            //                 // if we have an actual response - send it through the mixnet!
            //                 response_sender_clone
            //                     .unbounded_send((response, return_address))
            //                     .expect("channel got closed?");
            //             }
            //         };
            //     });
            // }

            // let controller = Controller::new();
            //
            // let (sender, mut reader) = mpsc::unbounded::<(Response, Recipient)>();

            /*
            // direct
                websocket_writer -> writes MIXNET requests
                websocket_reader ->

             // indirect
                sender ->
                reader -> reads MIXNET requests


                websocket_reader -> CONTROLLER -> connection <PROCESSING HERE> -> sender -> ...
                sender => reader -> websocket_writer
             */

            // TODO:
            // for proxy:
            /*
                controller will grab correct connection and send from reader to tcp writer
                from there we can wreite response to sender

            */
            //
            // let response_reader_future = async move {
            //     // TODO: wire SURBs in here once they're available
            //     while let Some((response, return_address)) = reader.next().await {
            //         // make 'request' to native-websocket client
            //         let response_message = ClientRequest::Send {
            //             recipient: return_address,
            //             message: response.into_bytes(),
            //             with_reply_surb: false,
            //         };
            //
            //         let message = Message::Binary(response_message.serialize());
            //         websocket_writer.send(message).await.unwrap();
            //     }
            // };
            // tokio::spawn(response_reader_future);
            //
            // println!("\nAll systems go. Press CTRL-C to stop the server.");
            // while let Some(msg) = websocket_reader.next().await {
            //     let data = msg.unwrap().into_data();
            //     let received = match ServerResponse::deserialize(&data).expect("todo: error handling") {
            //         ServerResponse::Received(received) => received,
            //         ServerResponse::Error(err) => {
            //             panic!("received error from native client! - {}", err)
            //         }
            //         _ => unimplemented!("probably should never be reached?"),
            //     };
            //
            //     let raw_message = received.message;
            //     let request = Request::try_from_bytes(&raw_message).unwrap();
            //
            //     let mut controller_local_pointer = controller.clone();
            //     let response_sender_clone = sender.clone();
            //
            //     // response_sender_clone is ALMOST our mix_sender
            //     // TODO: spawn a task with some channels?
            //
            //     // process_connect()
            //     // run_proxy()
            //     // something close
            //
            //
            //     tokio::spawn(async move {
            //         if let Ok(response_option) = controller_local_pointer.process_request(request).await
            //         {
            //             if let Some((response, return_address)) = response_option {
            //                 // if we have an actual response - send it through the mixnet!
            //                 response_sender_clone
            //                     .unbounded_send((response, return_address))
            //                     .expect("channel got closed?");
            //             }
            //         };
            //     });
        }
    }

    // /// Keep running until the user hits CTRL-C.
    // pub fn run_forever(&mut self) {
    //     if let Err(e) = self.runtime.block_on(tokio::signal::ctrl_c()) {
    //         println!("Stopping with error: {:?}", e);
    //     }
    //     println!("\nStopping...");
    // }

    // Make the websocket connection so we can receive incoming Mixnet messages.
    async fn connect_websocket(&mut self, uri: &str) -> WebSocketStream<TcpStream> {
        let ws_stream = match websocket::Connection::new(uri).connect().await {
            Ok(ws_stream) => {
                println!("* connected to local websocket server at {}", uri);
                ws_stream
            }
            Err(WebsocketConnectionError::ConnectionNotEstablished) => {
                panic!("Error: websocket connection attempt failed, is the Nym client running?")
            }
        };
        return ws_stream;
    }
}
