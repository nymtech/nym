package main

import (
	"fmt"
	"io/ioutil"
	"strings"

	"github.com/btcsuite/btcutil/base58"
	"github.com/gorilla/websocket"
)

func main() {
	uri := "ws://localhost:1977"

	conn, _, err := websocket.DefaultDialer.Dial(uri, nil)
	if err != nil {
		panic(err)
	}
	defer conn.Close()

	wallet := "4QC5D8auMbVpFVBfiZnVtQVUPiNUV9FMnpb81cauFpEp@GYCqU48ndXke9o2434i7zEGv1sWg1cNVswWJfRnY1VTB"
	splitAddress := strings.Split(wallet, "@")
	decodedDestination := base58.Decode(splitAddress[0])
	decodedGateway := base58.Decode(splitAddress[1])

	html := "<html><body>hi from go!</body></html>"

	readData := []byte(html)

	payload := append(decodedDestination[:], append(decodedGateway[:], read_data[:]...)...)
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
