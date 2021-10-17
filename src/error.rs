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
}
