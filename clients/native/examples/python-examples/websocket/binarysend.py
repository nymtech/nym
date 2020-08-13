import asyncio
import base58
import json
import websockets

self_address_request = json.dumps({
    "type": "selfAddress"
})

WITHOUT_REPLY: int = 0
WITH_REPLY: int = 1
REPLY: int = 2

# this is really flaky so ideally will be replaced by some proper serialization
# 16 - length of encryption key
# 32 - length of first hop
# 348 - current sphinx header size
# 192 - size of single payload key
# 4 - number of mix hops (gateway + 3 nodes)
REPLY_SURB_LEN: int = 16 + 32 + 348 + 4 * 192

async def send_file_with_reply():
    uri = "ws://localhost:1977"
    async with websockets.connect(uri) as websocket:
        await websocket.send(self_address_request)
        self_address = json.loads(await websocket.recv())
        print("our address is: {}".format(self_address["address"]))
        # we receive our address in string format of OUR_ID_PUB_KEY . OUR_ENC_PUB_KEY @ OUR_GATE_ID_PUB_KEY
        # all keys are 32 bytes and we need to encode them as binary without the '.' or '@' signs
        split_address = self_address["address"].split("@")
        client_half = split_address[0]
        gateway_half = split_address[1]
        
        # to DH: I think at this point this should be replaced with some sort of protobuf / flatbuffers / Cap'n Proto, etc
        split_client_address = client_half.split(".")

        bin_payload = bytearray([WITH_REPLY])
        bin_payload += base58.b58decode(split_client_address[0])
        bin_payload += base58.b58decode(split_client_address[1])
        bin_payload += base58.b58decode(gateway_half)

        with open("dummy_file", "rb") as input_file:
            read_data = input_file.read()
            bin_payload += read_data

        print("sending content of 'dummy_file' over the mix network...")
        await websocket.send(bin_payload)

        print("waiting to receive the 'dummy_file' from the mix network...")
        received_data = await websocket.recv()
        assert received_data[0] == WITH_REPLY
        reply_surb = received_data[1:1 +REPLY_SURB_LEN]
        output_file_data = received_data[1 + REPLY_SURB_LEN:]

        with open("received_file_withreply", "wb") as output_file:
            print("writing the file back to the disk!")
            output_file.write(output_file_data)

        
        reply_message = b"hello from reply SURB! - thanks for sending me the file!"
        binary_reply = bytearray([REPLY])
        binary_reply += reply_surb
        binary_reply += reply_message

        print("sending '{}' (using reply SURB!) over the mix network...".format(reply_message))
        await websocket.send(binary_reply)

        print("waiting to receive a message from the mix network...")
        received_reply = await websocket.recv()
        assert received_reply[0] == WITHOUT_REPLY

        print("received '{}' from the mix network".format(received_reply[1:]))


            
async def send_file_without_reply():
    uri = "ws://localhost:1977"
    async with websockets.connect(uri) as websocket:
        await websocket.send(self_address_request)
        self_address = json.loads(await websocket.recv())
        print("our address is: {}".format(self_address["address"]))
        # we receive our address in string format of OUR_ID_PUB_KEY . OUR_ENC_PUB_KEY @ OUR_GATE_ID_PUB_KEY
        # all keys are 32 bytes and we need to encode them as binary without the '.' or '@' signs
        split_address = self_address["address"].split("@")
        client_half = split_address[0]
        gateway_half = split_address[1]
        
        # to DH: I think at this point this should be replaced with some sort of protobuf / flatbuffers / Cap'n Proto, etc
        split_client_address = client_half.split(".")

        bin_payload = bytearray([WITHOUT_REPLY])
        bin_payload += base58.b58decode(split_client_address[0])
        bin_payload += base58.b58decode(split_client_address[1])
        bin_payload += base58.b58decode(gateway_half)

        with open("dummy_file", "rb") as input_file:
            read_data = input_file.read()
            bin_payload += read_data

        print("sending content of 'dummy_file' over the mix network...")
        await websocket.send(bin_payload)

        print("waiting to receive the 'dummy_file' from the mix network...")
        received_data = await websocket.recv()
        assert received_data[0] == WITHOUT_REPLY
        with open("received_file_noreply", "wb") as output_file:
            print("writing the file back to the disk!")
            output_file.write(received_data[1:])

# asyncio.get_event_loop().run_until_complete(send_file_without_reply())
asyncio.get_event_loop().run_until_complete(send_file_with_reply())
