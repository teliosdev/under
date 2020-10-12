#[derive(thiserror::Error, Debug)]
pub enum ShortError {
    #[error("could not parse the given string ({:?}) as an address", .0)]
    InvalidAddress(String),
    #[error("an error occurred while attempting to construct routes")]
    RoutePatternConstructionError(regex::Error),
    #[error("could not serve server")]
    HyperServeError(hyper::Error),
    #[error("unknown error")]
    Unknown(#[source] anyhow::Error),
}
