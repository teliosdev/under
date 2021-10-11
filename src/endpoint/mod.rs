mod sync;

use crate::request::Request;
use crate::response::IntoResponse;
use crate::Response;
use std::future::Future;
use std::pin::Pin;

#[async_trait]
#[doc(notable_trait)]
/// An HTTP request handler.
///
/// This is automatically implemented for
/// `Fn(Request) -> impl Future<Output = impl IntoResponse>` types, but it may
/// be useful to implement this yourself.  All this is meant to do is be a
/// fallible function from a [`Request`] into a [`Response`].
///
/// [`Request`]: ../Request.struct.html
/// [`Response`]: ../Response.struct.html
pub trait Endpoint: Send + Sync + 'static {
    #[must_use]
    /// Transforms the request into the response.  However, a request may fail,
    /// and such a failure can be handled by down the stack.
    async fn apply(self: Pin<&Self>, request: Request) -> Result<Response, anyhow::Error>;
}

#[async_trait]
impl<Res, F, Fut> Endpoint for F
where
    F: Fn(Request) -> Fut + Sync + Send + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoResponse + Send + 'static,
{
    async fn apply(self: Pin<&Self>, request: Request) -> Result<Response, anyhow::Error> {
        self(request).await.into_response()
    }
}

/// Creates an endpoint that synchronously generates a response.
///
/// This does not spawn a blocking task; so any endpoint that uses this should
/// not block the task in its processing.  This is useful for endpoints that
/// quickly generate a response, or otherwise do not use futures.
///
/// ```rust
/// # #[tokio::main] fn main() -> Result<(), anyhow::Error> {
/// # let http = under::http();
/// http.at("/404").get(under::endpoint::sync(|_| {
///     under::Response::json(serde_json!({ "error": 404 }))
/// }));
/// # }
/// ```
pub fn sync<F, Res>(func: F) -> impl Endpoint
where
    F: Fn(Request) -> Res + Send + Sync + 'static,
    Res: IntoResponse + Send + 'static,
{
    self::sync::SyncEndpoint(func)
}

pub fn r#static<F>(func: F) -> impl Endpoint
where
    F: Fn() -> Response + Send + Sync + 'static,
{
    sync::<_, Result<Response, std::convert::Infallible>>(move |_| Ok(func()))
}

// // pub fn dir<P>(path: P) -> impl Endpoint
// // where
// //     P: Into<std::path::PathBuf>,
// // {
// //     self::dir::DirEndpoint::new(path)
// // }
