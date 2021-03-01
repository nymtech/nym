package main

import (
	"encoding/binary"
	"fmt"
	"io/ioutil"

	"github.com/gorilla/websocket"
)

// request tags
const SEND_REQUEST_TAG = 0x00
const REPLY_REQUEST_TAG = 0x01
const SELF_ADDRESS_REQUEST_TAG = 0x02

// response tags
const ERROR_RESPONSE_TAG = 0x00
const RECEIVED_RESPONSE_TAG = 0x01
const SELF_ADDRESS_RESPONSE_TAG = 0x02

func makeSelfAddressRequest() []byte {
	return []byte{SELF_ADDRESS_REQUEST_TAG}
}

func parseSelfAddressResponse(rawResponse []byte) []byte {
	if len(rawResponse) != 97 || rawResponse[0] != SELF_ADDRESS_RESPONSE_TAG {
		panic("Received invalid response")
	}
	return rawResponse[1:]
}

func makeSendRequest(recipient []byte, message []byte, withReplySurb bool) []byte {
	messageLen := make([]byte, 8)
	binary.BigEndian.PutUint64(messageLen, uint64(len(message)))

	surbByte := byte(0)
	if withReplySurb {
		surbByte = 1
	}

	out := []byte{SEND_REQUEST_TAG, surbByte}
	out = append(out, recipient...)
	out = append(out, messageLen...)
	out = append(out, message...)

	return out
}

func makeReplyRequest(message []byte, replySURB []byte) []byte {
	messageLen := make([]byte, 8)
	binary.BigEndian.PutUint64(messageLen, uint64(len(message)))

	surbLen := make([]byte, 8)
	binary.BigEndian.PutUint64(surbLen, uint64(len(replySURB)))

	out := []byte{REPLY_REQUEST_TAG}
	out = append(out, surbLen...)
	out = append(out, replySURB...)
	out = append(out, messageLen...)
	out = append(out, message...)

	return out
}

func parseReceived(rawResponse []byte) ([]byte, []byte) {
	if rawResponse[0] != RECEIVED_RESPONSE_TAG {
		panic("Received invalid response!")
	}

	hasSurb := false
	if rawResponse[1] == 1 {
		hasSurb = true
	} else if rawResponse[1] == 0 {
		hasSurb = false
	} else {
		panic("malformed received response!")
	}

	data := rawResponse[2:]
	if hasSurb {
		surbLen := binary.BigEndian.Uint64(data[:8])
		other := data[8:]

		surb := other[:surbLen]
		msgLen := binary.BigEndian.Uint64(other[surbLen : surbLen+8])

		if len(other[surbLen+8:]) != int(msgLen) {
			panic("invalid msg len")
		}

		msg := other[surbLen+8:]
		return msg, surb
	} else {
		msgLen := binary.BigEndian.Uint64(data[:8])
		other := data[8:]

		if len(other) != int(msgLen) {
			panic("invalid msg len")
		}

		msg := other[:msgLen]
		return msg, nil
	}
}

func sendWithoutReply() {
	uri := "ws://localhost:1977"

	conn, _, err := websocket.DefaultDialer.Dial(uri, nil)
	if err != nil {
		panic(err)
	}
	defer conn.Close()

	selfAddressRequest := makeSelfAddressRequest()
	if err = conn.WriteMessage(websocket.BinaryMessage, selfAddressRequest); err != nil {
		panic(err)
	}
	_, receivedResponse, err := conn.ReadMessage()
	if err != nil {
		panic(err)
	}
	selfAddress := parseSelfAddressResponse(receivedResponse)

	read_data, err := ioutil.ReadFile("dummy_file")
	if err != nil {
		panic(err)
	}

	sendRequest := makeSendRequest(selfAddress, read_data, false)
	fmt.Printf("sending content of 'dummy file' over the mix network...\n")
	if err = conn.WriteMessage(websocket.BinaryMessage, sendRequest); err != nil {
		panic(err)
	}

	fmt.Printf("waiting to receive a message from the mix network...\n")
	_, receivedResponse, err = conn.ReadMessage()
	if err != nil {
		panic(err)
	}

	fileData, replySURB := parseReceived(receivedResponse)
	if replySURB != nil {
		panic("did not expect a replySURB!")
	}
	fmt.Printf("writing the file back to the disk!\n")
	ioutil.WriteFile("received_file_noreply", fileData, 0644)
}

func sendWithReply() {
	uri := "ws://localhost:1977"

	conn, _, err := websocket.DefaultDialer.Dial(uri, nil)
	if err != nil {
		panic(err)
	}
	defer conn.Close()

	selfAddressRequest := makeSelfAddressRequest()
	if err = conn.WriteMessage(websocket.BinaryMessage, selfAddressRequest); err != nil {
		panic(err)
	}
	_, receivedResponse, err := conn.ReadMessage()
	if err != nil {
		panic(err)
	}
	selfAddress := parseSelfAddressResponse(receivedResponse)

	readData, err := ioutil.ReadFile("dummy_file")
	if err != nil {
		panic(err)
	}

	sendRequest := makeSendRequest(selfAddress, readData, true)
	fmt.Printf("sending content of 'dummy file' over the mix network...\n")
	if err = conn.WriteMessage(websocket.BinaryMessage, sendRequest); err != nil {
		panic(err)
	}

	fmt.Printf("waiting to receive a message from the mix network...\n")
	_, receivedResponse, err = conn.ReadMessage()
	if err != nil {
		panic(err)
	}

	fileData, replySURB := parseReceived(receivedResponse)

	fmt.Printf("writing the file back to the disk!\n")
	ioutil.WriteFile("received_file_withreply", fileData, 0644)

	replyMessage := []byte("hello from reply SURB! - thanks for sending me the file!")
	replyRequest := makeReplyRequest(replyMessage, replySURB)

	fmt.Printf("sending '%v' (using reply SURB) over the mix network...\n", string(replyMessage))
	if err = conn.WriteMessage(websocket.BinaryMessage, replyRequest); err != nil {
		panic(err)
	}

	fmt.Printf("waiting to receive a message from the mix network...\n")
	_, receivedResponse, err = conn.ReadMessage()
	if err != nil {
		panic(err)
	}

	receivedMessage, replySURB := parseReceived(receivedResponse)
	if replySURB != nil {
		panic("did not expect a replySURB!")
	}

	fmt.Printf("received %v from the mix network!\n", string(receivedMessage))

}

func main() {
	// sendWithoutReply()
	sendWithReply()
}
