package main

import (
	"encoding/json"
	"fmt"
	"github.com/btcsuite/btcutil/base58"
	"github.com/gorilla/websocket"
	"io/ioutil"
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

func main() {
	uri := "ws://localhost:1977"

	conn, _, err := websocket.DefaultDialer.Dial(uri, nil)
	if err != nil {
		panic(err)
	}
	defer conn.Close()

	selfAddress := getSelfAddress(conn)
	fmt.Printf("our address is: %v\n", selfAddress)
	decodedAddress := base58.Decode(selfAddress)

	read_data, err := ioutil.ReadFile("dummy_file")
	if err != nil {
		panic(err)
	}

	payload := append(decodedAddress[:], read_data[:]...)
	fmt.Printf("sending content of 'dummy file' over the mix network...\n")
	if err = conn.WriteMessage(websocket.BinaryMessage, payload); err != nil {
		panic(err)
	}
	sendConfirmationJSON := make(map[string]interface{})
	err = conn.ReadJSON(&sendConfirmationJSON)
	if err != nil {
		panic(err)
	}
	if sendConfirmationJSON["type"].(string) != "send" {
		panic("invalid send confirmation")
	}

	fmt.Printf("waiting to receive a message from the mix network...\n")
	_, receivedMessage, err := conn.ReadMessage()
	if err != nil {
		panic(err)
	}

	fmt.Printf("writing the file back to the disk!\n")
	ioutil.WriteFile("received_file", receivedMessage, 0644)
}
