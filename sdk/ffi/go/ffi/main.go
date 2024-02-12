package main

import (
	"fmt"
	"nymffi/ffi/bindings"
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
		fmt.Println("Error:", err2)
		return
	}
	fmt.Println("response:")
	fmt.Println(str)

	// send a message, in this case to ourselves
	err3 := bindings.SendMessage(str, "helloworld")
	if err3 != nil {
		fmt.Println("Error:", err3)
		return
	}

	// assuming a data type like so:
	//type IncomingMessage struct {
	//	Message   string
	//	SenderTag string
	//}
	incomingMessage, err4 := bindings.ListenForIncoming()
	if err4 != nil {
		fmt.Println("Error:", err4)
		return
	}
	fmt.Println("incoming message: ", incomingMessage.Message, " from: ", incomingMessage.Sender)

	err5 := bindings.Reply("replyworld", incomingMessage.Sender)
	if err5 != nil {
		fmt.Println("Error:", err5)
		return
	}

	fmt.Println("end go example")
	time.Sleep(30 * time.Second)
}
