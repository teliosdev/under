//! Pre-defined endpoints.
//!
//! This module defines a few endpoints that might be useful for a given HTTP
//! application.  Their use should be as simple as this:
//!
//! ```rust
//! # use under::*;
//! # #[tokio::main] async fn main() -> Result<(), anyhow::Error> {
//! # let mut http = under::http();
//! http.at("/home").get(under::endpoints::simple(|| {
//!     Response::text("hello, there!")
//! }));
//! # Ok(())
//! # }
//! ```

mod dir;
mod scope;
mod sync;

pub use self::scope::{ScopeEndpoint, ScopeEndpointBuilder};
pub(crate) use self::sync::SyncEndpoint;
use crate::response::IntoResponse;
use crate::{Endpoint, Request};

/// Creates an endpoint that synchronously generates a response.
///
/// This does not spawn a blocking task; so any endpoint that uses this should
/// not block the task in its processing.  This is useful for endpoints that
/// quickly generate a response, or otherwise do not use futures.
///
/// # Examples
///
/// ```rust
/// # #[tokio::main] async fn main() -> Result<(), anyhow::Error> {
/// let mut http = under::http();
/// http.at("/404").get(under::endpoints::sync(|_| {
///     under::Response::json(&serde_json::json!({ "error": 404 }))
/// }));
/// # Ok(())
/// # }
/// ```
pub fn sync<F, Res>(func: F) -> impl Endpoint
where
    F: Fn(Request) -> Res + Send + Sync + 'static,
    Res: IntoResponse + Send + 'static,
{
    self::sync::SyncEndpoint(func)
}

/// Creates an endpoint that synchronously, infallibly generates a response.
///
/// This is meant for a very basic operation that returns a specific response
/// regardless of the request.  This is best paired with a
/// [`crate::Response::empty_404`]-like function.
///
/// This, like [`sync()`], does not spawn a blocking task; so any endpoint that
/// uses this should not block the task in its processing.  This is useful for
/// endpoints that quickly generate a response, or otherwise do not use futures.
///
/// # Examples
///
/// ```rust
/// # #[tokio::main] async fn main() -> Result<(), anyhow::Error> {
/// let mut http = under::http();
/// http.at("/404").get(under::endpoints::simple(under::Response::empty_404));
/// # Ok(())
/// # }
/// ```
pub fn simple<F, Res>(func: F) -> impl Endpoint
where
    F: Fn() -> Res + Send + Sync + 'static,
    Res: IntoResponse + Send + 'static,
{
    sync::<_, Res>(move |_| func())
}

/// Creates an endpoint that serves files from the given directory.
///
/// The endpoint expects the path to use to be a part of the request fragment
/// from the route; i.e., the route must have a pattern in it, like
/// `/public/{:path}`.  The name itself does not matter, as the endpoint
/// retrieves the first match.  Thus, `/users/{id}/files/{:path}` will not work
/// with this endpoint.  If there is a use case for this pattern, please file
/// a github ticket.
///
/// The endpoint will guess the Content-Type based off of the extension, or
/// default to `application/octet-stream` if it cannot be guessed.
///
/// If the router pattern is misconfigured, it will 404; if the file path
/// contains any segment consisting of `".."`, it will 404; if the file path
/// contains any backslashes, it will 404; f the requested file refers to
/// a directory, but does not contain a terminating slash, it will permanently
/// redirect to the URL with the terminating slash; if the requested file
/// refers to a directory (and contains a terminating slash), it will attempt to
/// read `index.html` in that directory instead; if it cannot find the file,
/// it will 404; if it cannot read the file, it will 500; and finally, it will
/// attempt to stream the file with a 200.
///
/// # Examples
///
/// ```rust
/// # #[tokio::main] async fn main() -> Result<(), anyhow::Error> {
/// # let mut http = under::http();
/// http.at("/public/{:path}").get(under::endpoints::dir("public/"));
/// # Ok(())
/// # }
/// ```
pub fn dir<P>(path: P) -> impl Endpoint
where
    P: Into<std::path::PathBuf>,
{
    self::dir::DirEndpoint::new(path)
}

/// Creates a builder for a [`ScopeEndpoint`].
///
/// A [`ScopeEndpoint`] is an endpoint with attentional middleware in front
/// of it.  This middleware acts just as if it was a part of the normal
/// middleware stack just in front of the endpoint, but the middleware in
/// the scope endpoint will always execute _after_ the middleware of whatever
/// router the endpoint is in front of.
///
/// This could be useful for (for example) restricting a subset of routes to
/// being authorization-restricted.
///
/// # Examples
/// ```rust,no_run
/// # use under::*;
/// # #[tokio::main] async fn main() -> Result<(), anyhow::Error> {
/// let mut http = under::http();
///
/// async fn endpoint(request: Request) -> Response {
///     let target = request.state::<String>().map(|v| v.as_str()).unwrap_or("world");
///     Response::text(format!("hello, {}", target))
/// }
///
/// http.at("/foo").get(endpoint);
/// http.at("/bar").get(under::endpoints::scope()
///     .with(under::middleware::StateMiddleware::new("bar".to_string()))
///     .then(endpoint));
/// http.prepare();
/// let mut response = http.handle(Request::get("/foo")?).await?;
/// let body = response.data(512).into_text().await?;
/// assert_eq!(body, "hello, world");
/// let mut response = http.handle(Request::get("/bar")?).await?;
/// let body = response.data(512).into_text().await?;
/// assert_eq!(body, "hello, bar");
/// # Ok(())
/// # }
/// ```
pub fn scope() -> ScopeEndpointBuilder {
    Default::default()
}
