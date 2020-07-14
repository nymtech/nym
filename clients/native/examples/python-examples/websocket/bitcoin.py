import asyncio
import websockets
import socket
import base58
from struct import *
import json

# An example Nym service provider peap which proxies incoming Bitcoin requests
# from the Nym network, and forwards the requests to a Bitcoin node.

# Install pipenv, then run it with `pipenv shell && python bitcoin.py`


def print_bytes(byte_array):
    for value in byte_array:
        print(value)


async def bitcoin_proxy():
    nym_client_uri = "ws://localhost:1977"
    async with websockets.connect(nym_client_uri) as websocket:
        print("waiting to receive a message from the mix network...")
        while True:
            received_message = await websocket.recv()
            # print("received WEBSOCKET message: {}".format(received_message))
            # received_message is a blob of bytes serialized like this:
            # || 2 bytes address len || address || 16 bytes request_id || message
            # print("received serialized payload length: {}".format(
            #     len(received_message)))
            address_length_bytes = received_message[0:2]
            # print("address length bytes: {}".format(address_length_bytes))

            address_length = int.from_bytes(
                address_length_bytes, byteorder='big', signed=False)
            # print("len: {}".format(address_length))

            address_bytes = received_message[2:2+address_length]
            address = address_bytes.decode('utf-8')
            print("address: {}".format(address))

            request_id = received_message[2+address_length:2+address_length+16]
            # print_bytes(request_id)

            tcp_request = received_message[2+address_length+16:]

            print("opening socket...")
            client = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            try:
                client.settimeout(15)
            except OSError:
                print("caught timeout")
                continue
            ip_host = address.split(":")
            try:
                client.connect((ip_host[0], int(ip_host[1])))
            except socket.error as exc:
                print("caught socket connect error: {}".format(exc))

            try:
                client.send(tcp_request)
            except socket.error as exc:
                print("caught socket send error: {}".format(exc))

            response = []

            while True:
                buf_size = 65536
                try:
                    data_chunk = client.recv(buf_size)
                    response += data_chunk
                except socket.error as exc:
                    print("recv error: {}".format(exc))
                    break
                except TypeError as exc:
                    print("type error in while loop: {}".format(exc))
                    break
                if len(data_chunk) != buf_size:
                    socket.close(0)
                    print("socket should get closed")
                    break

            print("whole response: {}", format(response))
            wallet = "4QC5D8auMbVpFVBfiZnVtQVUPiNUV9FMnpb81cauFpEp@GYCqU48ndXke9o2434i7zEGv1sWg1cNVswWJfRnY1VTB"
            split_address = wallet.split("@")
            bin_payload = bytearray(base58.b58decode(split_address[0]))
            bin_payload += base58.b58decode(split_address[1])
            try:
                bin_payload += bytearray(request_id)
            except TypeError as exc:
                print("type error while building bytearray response: {}".format(exc))
                continue

            bin_payload += bytearray(response)

            try:
                await websocket.send(bin_payload)
            except error as err:
                print("error sending to websocket: {}".format(err))
                continue

            # { "type" : "send" }
            try:
                msg_send_confirmation = json.loads(await websocket.recv())
                assert msg_send_confirmation["type"], "send"
            except error as err:
                print("caught json error: {}".format(err))


asyncio.get_event_loop().run_until_complete(bitcoin_proxy())
