
import asyncio
import base58
import json
import websockets

self_address_request = json.dumps({
    "type": "selfAddress"
})


async def send_file():
    uri = "ws://localhost:1977"
    async with websockets.connect(uri) as websocket:
        await websocket.send(self_address_request)
        self_address = json.loads(await websocket.recv())
        print("our address is: {}".format(self_address["address"]))

        bin_payload = bytearray(base58.b58decode(self_address["address"]))
        with open("dummy_file", "rb") as input_file:
            read_data = input_file.read()
            bin_payload += read_data

        print("sending content of 'dummy_file' over the mix network...")
        await websocket.send(bin_payload)
        msg_send_confirmation = json.loads(await websocket.recv())
        assert msg_send_confirmation["type"], "send"

        print("waiting to receive the 'dummy_file' from the mix network...")
        received_data = await websocket.recv()
        with open("received_file", "wb") as output_file:
            print("writing the file back to the disk!")
            output_file.write(received_data)

asyncio.get_event_loop().run_until_complete(send_file())
