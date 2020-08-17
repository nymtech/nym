package main

import (
	"encoding/json"
	"fmt"

	"github.com/gorilla/websocket"
)

func getSelfAddress(conn *websocket.Conn) string {
	selfAddressRequest, err := json.Marshal(map[string]string{"type": "selfAddress"})
	if err != nil {
		panic(err)
	}

	if err = conn.WriteMessage(websocket.TextMessage, []byte(selfAddressRequest)); err != nil {
		panic(err)
	}

	responseJSON := make(map[string]interface{})
	err = conn.ReadJSON(&responseJSON)
	if err != nil {
		panic(err)
	}

	return responseJSON["address"].(string)
}

func sendWithoutReply() {
	message := "Hello Nym!"

	uri := "ws://localhost:1977"

	conn, _, err := websocket.DefaultDialer.Dial(uri, nil)
	if err != nil {
		panic(err)
	}
	defer conn.Close()

	selfAddress := getSelfAddress(conn)
	fmt.Printf("our address is: %v\n", selfAddress)
	sendRequest, err := json.Marshal(map[string]interface{}{
		"type":          "send",
		"recipient":     selfAddress,
		"message":       message,
		"withReplySurb": false,
	})
	if err != nil {
		panic(err)
	}

	fmt.Printf("sending '%v' (*without* reply SURB) over the mix network...\n", message)
	if err = conn.WriteMessage(websocket.TextMessage, []byte(sendRequest)); err != nil {
		panic(err)
	}

	fmt.Printf("waiting to receive a message from the mix network...\n")
	_, receivedMessage, err := conn.ReadMessage()
	if err != nil {
		panic(err)
	}
	fmt.Printf("received %v from the mix network!\n", string(receivedMessage))
}

func sendWithReply() {
	message := "Hello Nym!"

	uri := "ws://localhost:1977"

	conn, _, err := websocket.DefaultDialer.Dial(uri, nil)
	if err != nil {
		panic(err)
	}
	defer conn.Close()

	selfAddress := getSelfAddress(conn)
	fmt.Printf("our address is: %v\n", selfAddress)
	sendRequest, err := json.Marshal(map[string]interface{}{
		"type":          "send",
		"recipient":     selfAddress,
		"message":       message,
		"withReplySurb": true,
	})
	if err != nil {
		panic(err)
	}

	fmt.Printf("sending '%v' (*with* reply SURB) over the mix network...\n", message)
	if err = conn.WriteMessage(websocket.TextMessage, []byte(sendRequest)); err != nil {
		panic(err)
	}

	fmt.Printf("waiting to receive a message from the mix network...\n")
	_, receivedMessage, err := conn.ReadMessage()
	if err != nil {
		panic(err)
	}
	fmt.Printf("received %v from the mix network!\n", string(receivedMessage))

	receivedMessageJson := make(map[string]interface{})
	if err := json.Unmarshal(receivedMessage, &receivedMessageJson); err != nil {
		panic(err)
	}

	// use the received surb to send an anonymous reply!
	replySURB := receivedMessageJson["replySURB"]
	replyMessage := "hello from reply SURB!"

	reply, err := json.Marshal(map[string]interface{}{
		"type":      "reply",
		"message":   replyMessage,
		"replySURB": replySURB,
	})
	if err != nil {
		panic(err)
	}

	fmt.Printf("sending '%v' (using reply SURB) over the mix network...\n", replyMessage)
	if err = conn.WriteMessage(websocket.TextMessage, []byte(reply)); err != nil {
		panic(err)
	}

	fmt.Printf("waiting to receive a message from the mix network...\n")
	_, receivedMessage, err = conn.ReadMessage()
	if err != nil {
		panic(err)
	}
	fmt.Printf("received %v from the mix network!\n", string(receivedMessage))
}

func main() {
	// sendWithoutReply()
	sendWithReply()
}
