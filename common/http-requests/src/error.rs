use thiserror::Error;

#[derive(Debug, Error)]
pub enum MixHttpRequestError {
    #[error("invalid Socks5 response")]
    InvalidSocks5Response,

    #[error("OrderedMessage error: {0}")]
    OrderedMessageError(#[from] nym_ordered_buffer::MessageError),

    #[error("bytecodec Error: {0}")]
    ByteCodecError(#[from] bytecodec::Error),

    #[error("Url parse error: {0}")]
    UrlParseError(#[from] url::ParseError),
}
