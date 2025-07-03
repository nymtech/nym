use super::mixnet_stream_wrapper::{MixSocket, MixStream, MixStreamReader, MixStreamWriter};
use futures::StreamExt;
// use nym_gateway_directory::IpPacketRouterAddress;
// use nym_ip_packet_requests::{
//     codec::MultiIpPacketCodec,
//     v8::{
//         request::{ControlRequest, IpPacketRequest, IpPacketRequestData},
//         response::{
//             ConnectResponse, ConnectResponseReply, ControlResponse, InfoLevel, IpPacketResponse,
//             IpPacketResponseData,
//         },
//     },
//     IpPair,
// };
use nym_sphinx::receiver::ReconstructedMessage;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, ReadBuf};
use tokio_util::codec::FramedRead;
use tracing::{debug, error, info, warn};
