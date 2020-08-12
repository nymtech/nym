package main

import (
	"encoding/json"
	"fmt"
	"io/ioutil"
	"strings"

	"github.com/btcsuite/btcutil/base58"
	"github.com/gorilla/websocket"
)

const WITHOUT_REPLY = 0
const WITH_REPLY = 1
const REPLY = 2

// this is really flaky so ideally will be replaced by some proper serialization
// 16 - length of encryption key
// 32 - length of first hop
// 348 - current sphinx header size
// 192 - size of single payload key
// 4 - number of mix hops (gateway + 3 nodes)
const REPLY_SURB_LEN = 16 + 32 + 348 + 4*192

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
	uri := "ws://localhost:1977"

	conn, _, err := websocket.DefaultDialer.Dial(uri, nil)
	if err != nil {
		panic(err)
	}
	defer conn.Close()

	selfAddress := getSelfAddress(conn)
	fmt.Printf("our address is: %v\n", selfAddress)

	// we receive our address in string format of OUR_ID_PUB_KEY . OUR_ENC_PUB_KEY @ OUR_GATE_ID_PUB_KEY
	// all keys are 32 bytes and we need to encode them as binary without the '.' or '@' signs
	splitAddress := strings.Split(selfAddress, "@")
	clientHalf := splitAddress[0]
	gatewayHalf := splitAddress[1]
	split_client_address := strings.Split(clientHalf, ".")

	decodedIdentity := base58.Decode(split_client_address[0])
	decodedEncryption := base58.Decode(split_client_address[1])
	decodedGateway := base58.Decode(gatewayHalf)

	read_data, err := ioutil.ReadFile("dummy_file")
	if err != nil {
		panic(err)
	}

	payload := []byte{WITHOUT_REPLY}
	payload = append(payload, decodedIdentity[:]...)
	payload = append(payload, decodedEncryption[:]...)
	payload = append(payload, decodedGateway[:]...)
	payload = append(payload, read_data[:]...)

	fmt.Printf("sending content of 'dummy file' over the mix network...\n")
	if err = conn.WriteMessage(websocket.BinaryMessage, payload); err != nil {
		panic(err)
	}

	fmt.Printf("waiting to receive a message from the mix network...\n")
	_, receivedMessage, err := conn.ReadMessage()
	if err != nil {
		panic(err)
	}
	if receivedMessage[0] != WITHOUT_REPLY {
		panic("incorrect prefix")
	}

	fmt.Printf("writing the file back to the disk!\n")
	ioutil.WriteFile("received_file_noreply", receivedMessage[1:], 0644)
}

func sendWithReply() {
	uri := "ws://localhost:1977"

	conn, _, err := websocket.DefaultDialer.Dial(uri, nil)
	if err != nil {
		panic(err)
	}
	defer conn.Close()

	selfAddress := getSelfAddress(conn)
	fmt.Printf("our address is: %v\n", selfAddress)

	// we receive our address in string format of OUR_ID_PUB_KEY . OUR_ENC_PUB_KEY @ OUR_GATE_ID_PUB_KEY
	// all keys are 32 bytes and we need to encode them as binary without the '.' or '@' signs
	splitAddress := strings.Split(selfAddress, "@")
	clientHalf := splitAddress[0]
	gatewayHalf := splitAddress[1]
	split_client_address := strings.Split(clientHalf, ".")

	decodedIdentity := base58.Decode(split_client_address[0])
	decodedEncryption := base58.Decode(split_client_address[1])
	decodedGateway := base58.Decode(gatewayHalf)

	read_data, err := ioutil.ReadFile("dummy_file")
	if err != nil {
		panic(err)
	}

	payload := []byte{WITH_REPLY}
	payload = append(payload, decodedIdentity[:]...)
	payload = append(payload, decodedEncryption[:]...)
	payload = append(payload, decodedGateway[:]...)
	payload = append(payload, read_data[:]...)

	fmt.Printf("sending content of 'dummy file' over the mix network...\n")
	if err = conn.WriteMessage(websocket.BinaryMessage, payload); err != nil {
		panic(err)
	}

	fmt.Printf("waiting to receive a message from the mix network...\n")
	_, receivedMessage, err := conn.ReadMessage()
	if err != nil {
		panic(err)
	}
	if receivedMessage[0] != WITH_REPLY {
		panic("incorrect prefix")
	}

	replySurb := receivedMessage[1 : 1+REPLY_SURB_LEN]
	outputFileData := receivedMessage[1+REPLY_SURB_LEN:]

	fmt.Printf("writing the file back to the disk!\n")
	ioutil.WriteFile("received_file_withreply", outputFileData, 0644)

	replyMessage := []byte("hello from reply SURB! - thanks for sending me the file!")
	binaryReply := []byte{REPLY}
	binaryReply = append(binaryReply, replySurb[:]...)
	binaryReply = append(binaryReply, replyMessage[:]...)

	fmt.Printf("sending '%v' (using reply SURB) over the mix network...\n", string(replyMessage))
	if err = conn.WriteMessage(websocket.BinaryMessage, binaryReply); err != nil {
		panic(err)
	}

	fmt.Printf("waiting to receive a message from the mix network...\n")
	_, receivedReply, err := conn.ReadMessage()
	if err != nil {
		panic(err)
	}

	if receivedReply[0] != WITHOUT_REPLY {
		panic("incorrect prefix")
	}

	fmt.Printf("received %v from the mix network!\n", string(receivedReply[1:]))

}

func main() {
	// sendWithoutReply()
	sendWithReply()
}
