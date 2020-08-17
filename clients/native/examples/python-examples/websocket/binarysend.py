import asyncio
import websockets
from pathlib import Path
import struct

# request tags
SEND_REQUEST_TAG = 0x00
REPLY_REQUEST_TAG = 0x01
SELF_ADDRESS_REQUEST_TAG = 0x02

# response tags
ERROR_RESPONSE_TAG = 0x00
RECEIVED_RESPONSE_TAG = 0x01
SELF_ADDRESS_RESPONSE_TAG = 0x02


def make_self_address_request() -> bytes:
    return bytes([SELF_ADDRESS_REQUEST_TAG])


def parse_self_address_response(raw_response: bytes) -> bytes:
    if len(raw_response) != 97 or raw_response[0] != SELF_ADDRESS_RESPONSE_TAG:
        print('Received invalid response!')
        raise

    return raw_response[1:]


def make_send_request(recipient: bytes, message: bytes, with_reply_surb: bool) -> bytes:
    # a big endian uint64
    message_len = len(message).to_bytes(length=8, byteorder='big', signed=False)

    return bytes([SEND_REQUEST_TAG]) + bytes([with_reply_surb]) + recipient + message_len + message


def make_reply_request(message: bytes, reply_surb: bytes) -> bytes:
    message_len = len(message).to_bytes(length=8, byteorder='big', signed=False)
    surb_len = len(reply_surb).to_bytes(length=8, byteorder='big', signed=False)

    return bytes([REPLY_REQUEST_TAG]) + surb_len + reply_surb + message_len + message


# it should have structure of RECEIVED_RESPONSE_TAG || with_reply || (surb_len || surb) || msg_len || msg
# where surb_len || surb is only present if 'with_reply' is true
def parse_received(raw_response: bytes) -> (bytes, bytes):
    if raw_response[0] != RECEIVED_RESPONSE_TAG:
        print('Received invalid response!')
        raise

    if raw_response[1] == 1:
        has_surb = True
    elif raw_response[1] == 0:
        has_surb = False
    else:
        print("malformed received response!")
        raise

    data = raw_response[2:]
    if has_surb:
        (surb_len,), other = struct.unpack(">Q", data[:8]), data[8:]
        surb = other[:surb_len]
        (msg_len,) = struct.unpack(">Q", other[surb_len:surb_len + 8])

        if len(other[surb_len + 8:]) != msg_len:
            print("invalid msg len")
            raise

        msg = other[surb_len + 8:]
        return msg, surb
    else:
        (msg_len,), other = struct.unpack(">Q", data[:8]), data[8:]
        if len(other) != msg_len:
            print("invalid msg len")
            raise

        msg = other[:msg_len]
        return msg, None


async def send_file_with_reply():
    uri = "ws://localhost:1977"
    async with websockets.connect(uri) as websocket:
        self_address_req = make_self_address_request()
        await websocket.send(self_address_req)

        self_address = parse_self_address_response(await websocket.recv())
        file_data = Path('dummy_file').read_bytes()

        send_request = make_send_request(self_address, file_data, True)
        print("sending content of 'dummy_file' over the mix network...")
        await websocket.send(send_request)

        print("waiting to receive the 'dummy_file' from the mix network...")
        received_response = await websocket.recv()
        received_file, surb = parse_received(received_response)

        with open("received_file_withreply", "wb") as output_file:
            print("writing the file back to the disk!")
            output_file.write(received_file)

        reply_message = b"hello from reply SURB! - thanks for sending me the file!"
        reply_request = make_reply_request(reply_message, surb)

        print("sending '{}' (using reply SURB!) over the mix network...".format(reply_message))
        await websocket.send(reply_request)

        print("waiting to receive a message from the mix network...")
        received_response = await websocket.recv()
        received_msg, surb = parse_received(received_response)
        assert surb is None  # no surbs in replies!

        print("received '{}' from the mix network".format(received_msg))


async def send_file_without_reply():
    uri = "ws://localhost:1977"
    async with websockets.connect(uri) as websocket:
        self_address_req = make_self_address_request()
        await websocket.send(self_address_req)

        self_address = parse_self_address_response(await websocket.recv())
        file_data = Path('dummy_file').read_bytes()

        send_request = make_send_request(self_address, file_data, False)
        print("sending content of 'dummy_file' over the mix network...")
        await websocket.send(send_request)

        print("waiting to receive the 'dummy_file' from the mix network...")
        received_response = await websocket.recv()
        received_file, surb = parse_received(received_response)
        assert surb is None  # we didn't attach a surb so we expect a None here!

        with open("received_file_noreply", "wb") as output_file:
            print("writing the file back to the disk!")
            output_file.write(received_file)


# asyncio.get_event_loop().run_until_complete(send_file_without_reply())
asyncio.get_event_loop().run_until_complete(send_file_with_reply())
