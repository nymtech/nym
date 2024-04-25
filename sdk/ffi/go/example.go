package main

import (
	"fmt"
	"nymffi/go-nym/bindings"
	"time"
)

func main() {

	// initialise Nym client logging - this is quite verbose but very informative
	bindings.InitLogging()

	// initialise an ephemeral client - aka one without specified keystore
	err := bindings.InitEphemeral()
	if err != nil {
		fmt.Println(err)
		return
	}

	// get our client's address
	str, err2 := bindings.GetSelfAddress()
	if err2 != nil {
		fmt.Println("(Go) Error:", err2)
		return
	}
	fmt.Println("(Go) response:")
	fmt.Println(str)

	// send a message through the mixnet - in this case to ourselves using the value from GetSelfAddress
	err3 := bindings.SendMessage(str, "helloworld")
	if err3 != nil {
		fmt.Println("(Go) Error:", err3)
		return
	}

	// listen out for incoming messages: in the future the client can be split into a listening and a sending client,
	// allowing for this to run as a persistent process in its own thread and not have to block but instead be running
	// concurrently
	//
	// assuming a data type like so:
	// type IncomingMessage struct {
	//  	Message   string
	//  	SenderTag vec<u8>
	// }
	incomingMessage, err4 := bindings.ListenForIncoming()
	if err4 != nil {
		fmt.Println("(Go) Error:", err4)
		return
	}
	fmt.Println("(Go) incoming message: ", incomingMessage.Message, " from: ", incomingMessage.Sender)

	// we can just use the byte array we parsed from the incoming message to reply with: this is a
	// byte representation of the sender_tag used for Single Use Reply Blocks (SURBs)
	//
	// replying to incoming message (from ourselves) with SURBs - note that sending a message to a recipient and
	// replying to an incoming are different functions: replying relies on parsing the incoming sender_tag on the Rust
	// side and creating an AnonymousSenderTag type, instead of the Recipient type which relies on a nym address
	//
	// you will see in the client logs that there are requests for more SURBs that we send to ourselves to
	// be able to fit the full reply message in there. In a future iteration of this code we can also expose
	// a send() which allows for developers to dictate the number of SURBs to send along with their outgoing message
	fmt.Println("(Go) replying to received message")
	err5 := bindings.Reply(incomingMessage.Sender, "replyworld")
	if err5 != nil {
		fmt.Println("(Go) Error:", err5)
		return
	}

	// sleep so that the nym client processes can catch up - in reality you'd have another process
	// running to keep logging going, so this is only necessary for this reference
	time.Sleep(30 * time.Second)
	fmt.Println("(Go) end go example")
}
