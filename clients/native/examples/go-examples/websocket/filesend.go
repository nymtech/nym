package main

import (
	"encoding/json"
	"fmt"
	"github.com/btcsuite/btcutil/base58"
	"github.com/gorilla/websocket"
	"io/ioutil"
	"strings"
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

    // we receive our address in string format of OUR_PUB_KEY @ OUR_GATE_PUB_KEY
    // both keys are 32 bytes and we need to encode them as binary without the '@' sign
    splitAddress := strings.Split(selfAddress, "@");
	decodedDestination := base58.Decode(splitAddress[0])
    decodedGateway := base58.Decode(splitAddress[1])

	read_data, err := ioutil.ReadFile("dummy_file")
	if err != nil {
		panic(err)
	}

	payload := append(decodedDestination[:], append(decodedGateway[:], read_data[:]...)...)
	fmt.Printf("sending content of 'dummy file' over the mix network...\n")
	if err = conn.WriteMessage(websocket.BinaryMessage, payload); err != nil {
		panic(err)
	}

	fmt.Printf("waiting to receive a message from the mix network...\n")
	_, receivedMessage, err := conn.ReadMessage()
	if err != nil {
		panic(err)
	}

	fmt.Printf("writing the file back to the disk!\n")
	ioutil.WriteFile("received_file", receivedMessage, 0644)
}
