import asyncio
import json
import websockets

self_address_request = json.dumps({
    "type": "selfAddress"
})


async def send_text_without_reply():
    message = "Hello Nym!"

    uri = "ws://localhost:1977"
    async with websockets.connect(uri) as websocket:
        await websocket.send(self_address_request)
        self_address = json.loads(await websocket.recv())
        print("our address is: {}".format(self_address["address"]))

        text_send = json.dumps({
            "type": "send",
            "message": message,
            "recipient": self_address["address"],
            "withReplySurb": False,
        })

        print("sending '{}' (*without* reply SURB) over the mix network...".format(message))
        await websocket.send(text_send)

        print("waiting to receive a message from the mix network...")
        received_message = await websocket.recv()
        print("received '{}' from the mix network".format(received_message))


async def send_text_with_reply():
    message = "Hello Nym!"

    uri = "ws://localhost:1977"
    async with websockets.connect(uri) as websocket:
        await websocket.send(self_address_request)
        self_address = json.loads(await websocket.recv())
        print("our address is: {}".format(self_address["address"]))

        text_send = json.dumps({
            "type": "send",
            "message": message,
            "recipient": self_address["address"],
            "withReplySurb": True,
        })

        print("sending '{}' (*with* reply SURB) over the mix network...".format(message))
        await websocket.send(text_send)

        print("waiting to receive a message from the mix network...")
        received_message = json.loads(await websocket.recv())
        print("received '{}' from the mix network".format(received_message))

        # use the received surb to send an anonymous reply!
        reply_surb = received_message["replySurb"]

        reply_message = "hello from reply SURB!"
        reply = json.dumps({
            "type": "reply",
            "message": reply_message,
            "replySurb": reply_surb
        })

        print("sending '{}' (using reply SURB!) over the mix network...".format(reply_message))
        await websocket.send(reply)

        print("waiting to receive a message from the mix network...")
        received_message = await websocket.recv()
        print("received '{}' from the mix network".format(received_message))



# asyncio.get_event_loop().run_until_complete(send_text_without_reply())
asyncio.get_event_loop().run_until_complete(send_text_with_reply())
