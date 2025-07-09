
## MixStream
WRITING (App → Mixnet):
1. App calls stream.write(bytes)
2. AsyncWrite implementation:
   - Deserializes bytes as InputMessage using InputMessageCodec
   - Forwards InputMessage to MixnetClient
3. MixnetClient sends to mixnet

READING (Mixnet → App):
1. MixnetClient receives ReconstructedMessage from Mixnet
2. Stream implementation buffers messages
3. AsyncRead implementation:
   - Encodes ReconstructedMessage using ReconstructedMessageCodec
   - Returns encoded bytes to app

Flow: App bytes ↔ InputMessage/ReconstructedMessage ↔ Mixnet

## IP Client
CONNECTING:
1. Create MixnetClient
2. Send IpPacketRequest::Connect wrapped in InputMessage
3. Use wait_for_messages() to get ReconstructedMessage from Mixnet
4. Parse as IpPacketResponse::Connect
5. Extract allocated IPs

SENDING IP PACKETS:
1. App provides IP packet bytes
2. Wrap in IpPacketRequest::Data
3. Send as InputMessage through MixnetClient

RECEIVING IP PACKETS:
1. IprListener processes ReconstructedMessages
2. Parse as IpPacketResponse
3. Extract IP packets from IpPacketResponse::Data using MultiIpPacketCodec
4. Return raw IP packet bytes to app

Flow: IP packets → IpPacketRequest → InputMessage → Mixnet → IPR
      IPR → Mixnet → IpPacketResponse → IP packets

## IPRMixStream
SETUP:
1. Create MixStream to IPR address
2. Split into reader/writer
3. Background task owns reader, processes IpPacketResponse messages
4. Main struct owns writer for sending

CONNECTING
1. Send IpPacketRequest::Connect wrapped in InputMessage
2. Use wait_for_messages() to get ReconstructedMessage from Mixnet
3. Parse as IpPacketResponse::Connect
4. Extract allocated IPs

WRITING (TUN → IPR?):
1. TUN device calls stream.write(ip_packet_bytes)
2. AsyncWrite implementation:
   - Wrap IP packet in IpPacketRequest::Data
   - Serialize to bytes using to_bytes()
   - Call writer.write_bytes() which:
     - Wraps in InputMessage
     - Sends through mixnet

READING (IPR → TUN):
1. Background task continuously reads from MixStreamReader
2. Decodes bytes as ReconstructedMessage
3. Parses as IpPacketResponse
4. Extracts IP packets from Data responses
5. Adds to shared buffer
6. AsyncRead polls buffer and returns IP packets to TUN

Flow:
Write: TUN IP packet → IpPacketRequest → serialize → MixStreamWriter → InputMessage → Mixnet → IPR
Read:  IPR → Mixnet → MixStreamReader → ReconstructedMessage → IpPacketResponse → extract IP packets → buffer → TUN
