# Troubleshooting
Below are several common issues or questions you may have. 

If you come across something that isn't explained here, [PRs are welcome](https://github.com/nymtech/nym/issues/new/choose). 

## Verbose `task client is being dropped` logging
### On client shutdown (expected)
If this is happening at the end of your code when disconnecting your client, this is fine; we just have a verbose client! When calling `client.disconnect().await` this is simply informing you that the client is shutting down.  

On client shutdown / disconnect this is to be expected - this can be seen in many of the code examples as well. We use the [`nym_bin_common::logging`](https://github.com/nymtech/nym/blob/develop/common/bin-common/src/logging/mod.rs) import to set logging in our example code. This defaults to `INFO` level.

If you wish to quickly lower the verbosity of your client process logs when developing you can prepend your command with `RUST_LOG=<LOGGING_LEVEL>`. 

If you want to run the `builder.rs` example with only `WARN` level logging and below:

```sh
cargo run --example builder 
```

Becomes:

```sh 
RUST_LOG=warn cargo run --example builder 
```

You can also make the logging _more_ verbose with: 

```sh
RUST_LOG=debug cargo run --example builder
```

### Not on client shutdown (unexpected) 
If this is happening unexpectedly then you might be shutting your client process down too early. See the [accidentally killing your client process](#accidentally-killing-your-client-process-too-early) below for possible explanations and how to fix this issue.  

[//]: # (TODO note on poisson dance and not immediately killing client process)
## Accidentally killing your client process too early
If you are seeing either of the following errors when trying to run a client, specifically sending a message, then you may be accidentally killing your client process. 

```sh
 2023-11-02T10:31:03.930Z INFO  TaskClient-BaseNymClient-real_traffic_controller-ack_control-action_controller                           > the task client is getting dropped
 2023-11-02T10:31:04.625Z INFO  TaskClient-BaseNymClient-received_messages_buffer-request_receiver                                       > the task client is getting dropped
 2023-11-02T10:31:04.626Z DEBUG nym_client_core::client::real_messages_control::acknowledgement_control::input_message_listener          > InputMessageListener: Exiting
 2023-11-02T10:31:04.626Z INFO  TaskClient-BaseNymClient-real_traffic_controller-ack_control-input_message_listener                      > the task client is getting dropped
 2023-11-02T10:31:04.626Z INFO  TaskClient-BaseNymClient-real_traffic_controller-reply_control                                           > the task client is getting dropped
 2023-11-02T10:31:04.626Z DEBUG nym_client_core::client::real_messages_control                                                           > The reply controller has finished execution!
 2023-11-02T10:31:04.626Z DEBUG nym_client_core::client::real_messages_control::acknowledgement_control                                  > The input listener has finished execution!
 2023-11-02T10:31:04.626Z INFO  nym_task::manager                                                                                        > All registered tasks succesfully shutdown
```

```sh
 2023-11-02T11:22:08.408Z ERROR TaskClient-BaseNymClient-topology_refresher                                                  > Assuming this means we should shutdown...
 2023-11-02T11:22:08.408Z ERROR TaskClient-BaseNymClient-mix_traffic_controller                                              > Polling shutdown failed: channel closed
 2023-11-02T11:22:08.408Z INFO  TaskClient-BaseNymClient-gateway_transceiver-child                                           > the task client is getting dropped
 2023-11-02T11:22:08.408Z ERROR TaskClient-BaseNymClient-mix_traffic_controller                                              > Assuming this means we should shutdown...
thread 'tokio-runtime-worker' panicked at 'action control task has died: TrySendError { kind: Disconnected }', /home/.local/share/cargo/git/checkouts/nym-fbd2f6ea2e760da9/a800cba/common/client-core/src/client/real_messages_control/message_handler.rs:634:14
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
 2023-11-02T11:22:08.477Z INFO  TaskClient-BaseNymClient-real_traffic_controller-ack_control-input_message_listener          > the task client is getting dropped
 2023-11-02T11:22:08.477Z ERROR TaskClient-BaseNymClient-real_traffic_controller-ack_control-input_message_listener          > Polling shutdown failed: channel closed
 2023-11-02T11:22:08.477Z ERROR TaskClient-BaseNymClient-real_traffic_controller-ack_control-input_message_listener          > Assuming this means we should shutdown...
```

Using the following piece of code as an example: 

```rust
use nym_sdk::mixnet::{MixnetClient, MixnetMessageSender, Recipient};
use clap::Parser;

#[derive(Debug, Clone, Parser)]
enum Opts {
    Client {
        recipient: Recipient
    }
}

#[tokio::main]
async fn main() {
    let opts: Opts = Parser::parse();
    nym_bin_common::logging::setup_logging();

    let mut nym_client = MixnetClient::connect_new().await.expect("Could not build Nym client");

    match opts {
        Opts::Client { recipient } => {
            nym_client.send_plain_message(recipient, "some message string").await.expect("send failed");
        }
    }
}
```

This is a simplified snippet of code for sending a simple hardcoded message with the following command: 

```sh
cargo run client <RECIPIENT_NYM_ADDRESS>
```

You might assume that `send`-ing your message would _just work_ as `nym_client.send_plain_message()` is an async function; you might expect that the client will block until the message is actually sent into the mixnet, then shutdown. 

However, this is not true. 

**This will only block until the message is put into client's internal queue**. Therefore in the above example, the client is being shut down before the message is _actually sent to the mixnet_; after being placed in the client's internal queue, there is still work to be done under the hood, such as route encrypting the message and placing it amongst the stream of cover traffic.  

The simple solution? Make sure the program/client stays active, either by calling `sleep`, or listening out for new messages. As sending a one-shot message without listening out for a response is likely not what you'll be doing, then you will be then awaiting a response (see the [message helpers page](message-helpers.md) for an example of this). 

Furthermore, you should always **manually disconnect your client** with `client.disconnect().await` as seen in the code examples. This is important as your client is writing to a local DB and dealing with SURB storage. 

## Client receives empty messages when listening for response
If you are sending out a message, it makes sense for your client to then listen out for incoming messages; this would probably be the reply you get from the service you've sent a message to. 

You might however be receiving messages without data attached to them / empty payloads. This is most likely because your client is receiving a message containing a [SURB request](https://nymtech.net/docs/architecture/traffic-flow.html#private-replies-using-surbs) - a SURB requesting more SURB packets to be sent to the service, in order for them to have enough packets (with a big enough overall payload) to split the entire response to your initial request across. 

Whether the `data` of a SURB request being empty is a feature or a bug is to be decided - there is some discussion surrounding whether we can use SURB requests to send additional data to streamline the process of sending large replies across the mixnet. 

You can find a few helper functions [here](message-helpers.md) to help deal with this issue in the meantime. 

> If you can think of a more succinct or different way of handling this do reach out - we're happy to hear other opinions 