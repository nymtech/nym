import asyncio
import websockets
import socket
import base58
from struct import *
import json

# An example Nym service provider peap which proxies incoming Bitcoin requests
# from the Nym network, and forwards the requests to a Bitcoin node.

# Install pipenv, then run it with `pipenv shell && python bitcoin.py`


async def bitcoin_proxy():
    nym_client_uri = "ws://localhost:1977"
    async with websockets.connect(nym_client_uri) as websocket:
        print("waiting to receive a message from the mix network...")
        while True:
            received_message = await websocket.recv()
            print("received WEBSOCKET message: {}".format(received_message))

            print("received len: {}".format(len(received_message)))
            len_bytes = received_message[0:2]
            print("len bytes: {}".format(len_bytes))

            foo = int.from_bytes(len_bytes, byteorder='big', signed=False)
            print("len: {}".format(foo))

            address_bytes = received_message[2:2+foo]
            address = address_bytes.decode('utf-8')
            print("address: {}".format(address))

            proxy_message = received_message[2+foo:]

            # 2 bytes address len || address || message

            print("\nreceived '{}' from the mix network".format(received_message))
            print("opening socket...")
            client = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            client.settimeout(5)
            ip_host = address.split(":")
            try:
                client.connect((ip_host[0], int(ip_host[1])))
            except socket.error as exc:
                print("caught socket connect error: {}".format(exc))

            try:
                client.send(proxy_message)
            except socket.error as exc:
                print("caught socket send error: {}".format(exc))

            response = []
            while True:
                buf_size = 1024
                try:
                    data_chunk = client.recv(buf_size)
                    response += data_chunk
                except socket.error as exc:
                    print("recv error: {}".format(exc))
                    break
                print(response)
                if len(data_chunk) != buf_size:
                    socket.close(0)
                    print("socket should get closed")
                    break
            print("whole response: {}", format(response))
            wallet = "4QC5D8auMbVpFVBfiZnVtQVUPiNUV9FMnpb81cauFpEp@GYCqU48ndXke9o2434i7zEGv1sWg1cNVswWJfRnY1VTB"
            split_address = wallet.split("@")
            bin_payload = bytearray(base58.b58decode(split_address[0]))
            bin_payload += base58.b58decode(split_address[1])
            bin_payload += bytearray(response)
            await websocket.send(bin_payload)

            msg_send_confirmation = json.loads(await websocket.recv())
            assert msg_send_confirmation["type"], "send"


asyncio.get_event_loop().run_until_complete(bitcoin_proxy())
