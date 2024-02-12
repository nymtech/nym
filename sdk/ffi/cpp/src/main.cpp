// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#include <iostream>
#include <thread>
#include <boost/chrono.hpp>
#include <boost/thread.hpp>

//Rust function & type signatures
extern "C" {
    struct ReceivedMessage {
       const uint8_t* message;
       size_t size;
       const char* sender_tag;
    };

    void* init_logging();
    char init_ephemeral();
    char get_self_address(void (*callback)(const char*));
    char send_message(const char*, const char*);
    char listen_for_incoming(void (*callback)(ReceivedMessage));
    char reply(const char*, const char*);
}

//return code for error handling
char return_code = 0;
// bytes for sender tag
char sender_tag[22];
// nym address
char addr[134];
// test messages
char message[14] = "Hello World";
char reply_message[14] = "Reply World";

void string_callback_function(const char* c_string) {
    std::cout << "(c++)  callback received: " << c_string << std::endl;
    strcpy(addr, c_string);
}

void incoming_message_callback(ReceivedMessage received) {
    // this is where you deal with the incoming message -
    // in this case we'll just log it and save sender_tag to a pre-allocated
    // buffer to reply to the message further down in main()
    std::cout << "(c++) sender tag: " << received.sender_tag << std::endl;
    std::cout << "(c++) message: " << received.message << std::endl;
    std::cout << "(c++) message length : " << received.size << std::endl;
    const char* incoming_sender_tag = received.sender_tag;
    strcpy(sender_tag, incoming_sender_tag);
}

// an overly simplified example - handle the error however you wish
int handle(char return_code) {
    if (return_code == 0) {
        return 0;
    } else {
        return -1;
    }
}

int main() {
    // initialise Nym client logging - this is quite verbose but very informative
    init_logging();

    // blocking thread example with error return code:
    // - package fn
    // - obtain as future
    // - execute
    // - get() returned val
    // - handle val
    boost::packaged_task<char> init(boost::bind(init_ephemeral));
    boost::unique_future<char> init_future = init.get_future();
    init();
    return_code = init_future.get();
    handle(return_code);

    // get_self_addr is sync so no thread required: this is the only exposed rust fn that isn't async
    return_code = get_self_address(string_callback_function);
    handle(return_code);

    // send a message through the mixnet - in this case to ourselves
    std::cout << "(c++)  message to send through mixnet: " << message << std::endl;
    boost::packaged_task<char> send(boost::bind(send_message, addr, message));
    boost::unique_future<char> send_future = send.get_future();
    send();
    return_code = send_future.get();
    handle(return_code);

    /*

    // listen out for incoming messages: in the future the client can be split into a listening and a sending client,
    // allowing for this to run as a persistent process in its own thread and not have to block but instead be running
    // concurrently
    boost::packaged_task<char> listen(boost::bind(listen_for_incoming, incoming_message_callback));
    boost::unique_future<char> listen_future = listen.get_future();
    listen();
    return_code = listen_future.get();
    handle(return_code);

    // replying to incoming message (from ourselves) with SURBs- note that sending a message to a recipient and
    // replying to an incoming are different functions
    boost::packaged_task<char> reply_fn(boost::bind(reply, sender_tag, reply_message));
    boost::unique_future<char> reply_future = reply_fn.get_future();
    reply_fn();
    return_code = reply_future.get();
    handle(return_code);

*/
    // sleep so that the nym side logging can catch up - in reality you'd have another process running to keep logging
    // going, so this is only necessary for this reference implementation
    std::this_thread::sleep_for(std::chrono::seconds(40));

    return 0;
}

