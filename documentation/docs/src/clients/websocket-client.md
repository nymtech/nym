# Websocket Client 


> The Nym Websocket Client was built in the [building nym](../binaries/building-nym.md) section. If you haven't yet built Nym and want to run the code on this page, go there first.


## Client setup 
### Viewing command help
  
You can check that your binaries are properly compiled with:

```
./nym-client --help
```

~~~admonish example collapsible=true title="Console output"
```

      _ __  _   _ _ __ ___
     | '_ \| | | | '_ \ _ \
     | | | | |_| | | | | | |
     |_| |_|\__, |_| |_| |_|
            |___/

             (client - version {{platform_release_version}})

    
      nym-client {{platform_release_version}}
      Nymtech
      Implementation of the Nym Client

      USAGE:
          nym-client [OPTIONS] <SUBCOMMAND>

      OPTIONS:
              --config-env-file <CONFIG_ENV_FILE>
                  Path pointing to an env file that configures the client

          -h, --help
                  Print help information

          -V, --version
                  Print version information

      SUBCOMMANDS:
          completions          Generate shell completions
          generate-fig-spec    Generate Fig specification
          help                 Print this message or the help of the given subcommand(s)
          init                 Initialise a Nym client. Do this first!
          run                  Run the Nym client with provided configuration client optionally
                               overriding set parameters
          upgrade              Try to upgrade the client


```
~~~

The two most important commands you will issue to the client are: 

* `init` - initalise a new client instance. 
* `run` - run a mixnet client process. 

You can check the necessary parameters for the available commands by running:

```
./nym-client <command> --help 
```

### Initialising your client 

Before you can use the client, you need to initalise a new instance of it. Each instance of the client has its own public/private keypair, and connects to its own gateway node. Taken together, these 3 things (public/private keypair + gateway node identity key) make up an app's identity.

Initialising a new client instance can be done with the following command:

```
./nym-client init --id <client_id> 
```

~~~admonish example collapsible=true title="Console output"
```
      Initialising client...
      Saved all generated keys
      Saved configuration file to "/home/mx/.nym/clients/client/config/config.toml"
      Using gateway: BNjYZPxzcJwczXHHgBxCAyVJKxN6LPteDRrKapxWmexv
      Client configuration completed.




      The address of this client is: 7bxykcEH1uGNMr8mxGABvLJA44nbYt6Rp7xXHhJ4wQVk.HpnFbaMJ8NN1cp5ZPdPTc2GoBDnG4Jd51Sti32tbf3tF@BNjYZPxzcJwczXHHgBxCAyVJKxN6LPteDRrKapxWmexv

```
~~~

The `--id` in the example above is a local identifier so that you can name your clients; it is **never** transmitted over the network.

There is an optional `--gateway` flag that you can use if you want to use a specific gateway. The supplied argument is the `Identity Key` of the gateway you wish to use, which can be found on the [mainnet Network Explorer](https://explorer.nymtech.net/network-components/gateways) or [Sandbox Testnet Explorer](https://sandbox-explorer.nymtech.net/network-components/gateways) depending on which network you are on. 

Not passing this argument will randomly select a gateway for your client.

#### Choosing a Gateway 
By default - as in the example above - your client will choose a random gateway to connect to. 

However, there are several options for choosing a gateway, if you do not want one that is randomly assigned to your client:
* If you wish to connect to a specific gateway, you can specify this with the `--gateway` flag when running `init`. 
* You can also choose a gateway based on its location relative to your client. This can be done by appending the `--latency-based-routing` flag to your `init` command. This command means that to select a gateway, your client will: 
	* fetch a list of all availiable gateways
	* send few ping messages to all of them, and measure response times. 
	* create a weighted distribution to randomly choose one, favouring ones with lower latency. 

> Note this doesn't mean that your client will pick the closest gateway to you, but it will be far more likely to connect to gateway with a 20ms ping rather than 200ms

### Configuring your client 
When you initalise a client instance, a configuration directory will be generated and stored in `$HOME_DIR/.nym/clients/<client-name>/`.

```
/home/<user>/.nym/clients/<client_id>/
├── config
│   └── config.toml
└── data
    ├── private_identity.pem
    └── public_identity.pem
```

The `config.toml` file contains client configuration options, while the two `pem` files contain client key information.

The generated files contain the client name, public/private keypairs, and gateway address. The name `<client_id>` in the example above is just a local identifier so that you can name your clients.

#### Configuring your client for Docker 
By default, the native client listens to host `127.0.0.1`. However this can be an issue if you wish to run a client in a Dockerized environment, where it can be convenenient to listen on a different host such as `0.0.0.0`. 

You can set this via the `--host` flag during either the `init` or `run` commands. 

Alternatively, a custom host can be set in the `config.toml` file under the `socket` section. If you do this, remember to restart your client process. 


### Running your client
You can run the initalised client by doing this:

```
./nym-client run --id <client_id>
```

When you run the client, it immediately starts generating (fake) cover traffic and sending it to the mixnet.

When the client is first started, it will reach out to the Nym network's validators, and get a list of available Nym nodes (gateways, mixnodes, and validators). We call this list of nodes the network _topology_. The client does this so that it knows how to connect, register itself with the network, and know which mixnodes it can route Sphinx packets through.

## Using your client 
### Connecting to the local websocket
The Nym native client exposes a websocket interface that your code connects to. To program your app, choose a websocket library for whatever language you're using. The **default** websocket port is `1977`, you can override that in the client config if you want.

The Nym monorepo includes websocket client example code for Rust, Go, Javacript, and Python, all of which can be found [here](https://github.com/nymtech/nym/tree/release/{{platform_release_version}}/clients/native/examples)

> Rust users can run the examples with `cargo run --example <rust_file>.rs`, as the examples are not organised in the same way as the other examples, due to already being inside a Cargo project. 


All of these code examples will do the following: 
* connect to a running websocket client on port `1977`
* format a message to send in either JSON or Binary format. Nym messages have defined JSON formats.
* send the message into the websocket. The native client packages the message into a Sphinx packet and sends it to the mixnet
* wait for confirmation that the message hit the native client
* wait to receive messages from other Nym apps

By varying the message content, you can easily build sophisticated service provider apps. For example, instead of printing the response received from the mixnet, your service provider might take some action on behalf of the user - perhaps initiating a network request, a blockchain transaction, or writing to a local data store.

> You can find an example of building both frontend and service provider code with the websocket client in the [Simple Service Provider Tutorial](https://nymtech.net/developers/tutorials/simple-service-provider.html) in the Developer Portal. 

### Message Types
There are a small number of messages that your application sends up the websocket to interact with the native client, as follows.

#### Sending text
If you want to send text information through the mixnet, format a message like this one and poke it into the websocket:

```json
{
  "type": "send",
  "message": "the message",
  "recipient": "71od3ZAupdCdxeFNg8sdonqfZTnZZy1E86WYKEjxD4kj@FWYoUrnKuXryysptnCZgUYRTauHq4FnEFu2QGn5LZWbm"
}
```

In some applications, e.g. where people are chatting with friends who they know, you might want to include unencrypted reply information in the message field. This provides an easy way for the receiving chat to then turn around and send a reply message:

```json
{
  "type": "send",
  "message": {
    "sender": "198427b63ZAupdCdxeFNg8sdonqfZTnZZy1E86WYKEjxD4kj@FWYoUrnKuXryysptnCZgUYRTauHq4FnEFu2QGn5LZWbm",
    "chatMessage": "hi julia!"
  },
  "recipient": "71od3ZAupdCdxeFNg8sdonqfZTnZZy1E86WYKEjxD4kj@FWYoUrnKuXryysptnCZgUYRTauHq4FnEFu2QGn5LZWbm"
}
```

If that fits your security model, good. However, it may be the case that you want to send **anonymous replies using Single Use Reply Blocks (SURBs)**. 

You can read more about SURBs [here](../architecture/traffic-flow.md#private-replies-using-surbs) but in short they are ways for the receiver of this message to anonymously reply to you - the sender - without them having to know your nym address. 

Your client will send along a number of `replySurbs` to the recipient of the message. These are pre-addressed Sphinx packets that the recipient can write to, but not view the address. If the recipient is unable to fit the response data into the bucket of SURBs sent to it, it will use a SURB to request more SURBs be sent to it. 

```json
{
    "type": "sendAnonymous", 
    "message": "something you want to keep secret"
    "recipient": "71od3ZAupdCdxeFNg8sdonqfZTnZZy1E86WYKEjxD4kj@FWYoUrnKuXryysptnCZgUYRTauHq4FnEFu2QGn5LZWbm"
    "replySurbs": 100 // however many reply SURBs to send along with your message
}
```

Each bucket of replySURBs, when received as part of an incoming message, has a unique session identifier, which **only identifies the bucket of pre-addressed packets**. This is necessary to make sure that your app is replying to the correct people with the information meant for them! Constructing a reply with SURBs looks something like this (where `senderTag` was parsed from the incoming message)

```json
{
    "type": "reply", 
    "message": "reply you also want to keep secret", 
    "senderTag": "the sender tag you parsed from the incoming message"
}
```


#### Sending binary data
You can also send bytes instead of JSON. For that you have to send a binary websocket frame containing a binary encoded
Nym [`ClientRequest`](https://github.com/nymtech/nym/blob/develop/clients/native/websocket-requests/src/requests.rs#L25) containing the same information. 

As a response the `native-client` will send a `ServerResponse` to be decoded. 

You can find examples of sending and receiving binary data in the Rust, Python and Go [code examples](https://github.com/nymtech/nym/tree/release/{{platform_release_version}}/clients/native/examples), and an example project from the Nym community [BTC-BC](https://github.com/sgeisler/btcbc-rs/): Bitcoin transaction transmission via Nym, a client and service provider written in Rust.

#### Getting your own address
Sometimes, when you start your app, it can be convenient to ask the native client to tell you what your own address is (from the saved configuration files). To do this, send:

```json
{
  "type": "selfAddress"
}
```

You'll get back:

```json
{
  "type": "selfAddress",
  "address": "the-address" // e.g. "71od3ZAupdCdxeFNg8sdonqfZTnZZy1E86WYKEjxD4kj@FWYoUrnKuXryysptnCZgUYRTauHq4FnEFu2QGn5LZWbm"
}
```

#### Error messages
Errors from the app's client, or from the gateway, will be sent down the websocket to your code in the following format:

```json
{
  "type": "error",
  "message": "string message"
}
```

