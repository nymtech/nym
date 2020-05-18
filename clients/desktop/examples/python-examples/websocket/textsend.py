import asyncio
import json
import websockets

self_address_request = json.dumps({
    "type": "selfAddress"
})

async def send_text():
    message = "Hello Nym!"

    uri = "ws://localhost:1977"
    async with websockets.connect(uri) as websocket:
        await websocket.send(self_address_request)
        self_address = json.loads(await websocket.recv())
        print("our address is: {}".format(self_address["address"]))

        text_send = json.dumps({
            "type": "send",
            "message": message,
            "recipient": self_address["address"]
        })

        print("sending '{}' over the mix network...".format(message))
        await websocket.send(text_send)
        msg_send_confirmation = json.loads(await websocket.recv())
        assert msg_send_confirmation["type"], "send"

        print("waiting to receive a message from the mix network...")
        received_message = await websocket.recv()
        print("received {} from the mix network!".format(received_message))

asyncio.get_event_loop().run_until_complete(send_text())
