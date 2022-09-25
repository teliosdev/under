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
    /// Generated when attempting to deserialize the body of a request or
    /// response from JSON.
    #[error("could not deserialize the body of a request or response from JSON")]
    JsonDeserialization(#[source] serde_json::Error),
    /// Generated when attempting to deserialize the body of a request or
    /// response from text.
    #[error("could not deserialize the body of a request or response from utf-8")]
    TextDeserialization(#[source] std::str::Utf8Error),
    #[cfg(feature = "from_form")]
    #[doc(cfg(feature = "from_form"))]
    /// Generated when attempting to deserialize the body of a request or
    /// response from x-www-form-urlencoded.
    #[error("could not deserialize the body of a request or response from urlencoded")]
    FormDeserialization(#[source] crate::from_form::FromFormError),
    /// Generated when attempting to sniff the request or response of its
    /// content type.
    #[error("the content-type of the request was invalid")]
    InvalidContentType(Option<mime::Mime>),
}
