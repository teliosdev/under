#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
/// Errors generated specifically from this library, and not its interactions
/// user code.
pub enum UnderError {
    #[error("could not parse the given string ({:?}) as an address", .0)]
    /// Generated when attempting to parse an address (during
    /// [`crate::Router::listen`]), but the address was invalid.
    InvalidAddress(String),
    #[error("could not serve server")]
    /// Generated when attempting to bind and listen using hyper, but it failed
    /// for some underlying reason.
    HyperServer(#[source] hyper::Error),
    /// Generated when attempting to read the body of a request, or response,
    /// and failing.
    #[error("could not read the body of a request or response")]
    ReadBody(#[source] std::io::Error),
    #[cfg(feature = "json")]
    #[doc(cfg(feature = "json"))]
    /// Generated when attempting to deserialize the body of a request or
    /// response from JSON.
    #[error("could not deserialize the body of a request or response from JSON")]
    JsonDeserialization(#[source] serde_json::Error),
    #[cfg(feature = "cbor")]
    #[doc(cfg(feature = "cbor"))]
    /// Generated when attempting to deserialize the body of a request or
    /// response from CBOR.
    #[error("could not deserialize the body of a request or response from CBOR")]
    CborDeserialization(#[source] anyhow::Error),
    #[cfg(feature = "msgpack")]
    #[doc(cfg(feature = "msgpack"))]
    /// Generated when attempting to deserialize the body of a request or
    /// response from MessagePack.
    #[error("could not deserialize the body of a request or response from MessagePack")]
    MsgpackDeserialization(#[source] rmp_serde::decode::Error),
    /// Generated when attempting to deserialize the body of a request or
    /// response from text.
    #[error("could not deserialize the body of a request or response from utf-8")]
    TextDeserialization(#[source] std::string::FromUtf8Error),
    #[cfg(feature = "from_form")]
    #[doc(cfg(feature = "from_form"))]
    /// Generated when attempting to deserialize the body of a request or
    /// response from x-www-form-urlencoded.
    #[error("could not deserialize the body of a request or response from urlencoded")]
    FormDeserialization(#[source] crate::from_form::FromFormError),
    /// Generated when attempting to sniff the request or response of its
    /// content type.
    #[error("the content-type of the request was invalid")]
    UnsupportedMediaType(Option<mime::Mime>),
    /// Generated when the request body of the request (if not provided with
    /// a Content-Length header) is too large.
    #[error("the request body of the request was too long, and was cut off")]
    PayloadTooLarge(#[source] anyhow::Error),
}
