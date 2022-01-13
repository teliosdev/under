//! Async SSE.
//!
//! This adds some wrappers around using the `async-sse` crate with this
//! HTTP library, making it easier to handle SSE connections.  It is gated
//! behind the `sse` feature flag for those who do not want to use it.

use crate::{Request, Response};
pub use async_sse::Sender;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio_util::compat::FuturesAsyncReadCompatExt;

/// Creates an endpoint that can handle SSE connections.  This directly
/// upgrades the HTTP request to SSE uncondintionally, before calling the
/// handler function with the current request and the SSE sender.
///
/// # Examples
/// ```rust,no_run
/// # use under::*;
/// use under::sse::Sender;
///
/// async fn handle(req: Request, mut sender: Sender) -> Result<(), anyhow::Error> {
///     sender.send(None, "hello, world!", None).await?;
///     Ok(())
/// }
///
/// let mut http = under::http();
/// http.at("/sse").get(under::sse::endpoint(handle));
/// ```
pub fn endpoint<F, Fut>(handle: F) -> SseEndpoint<F>
where
    F: Fn(Request, Sender) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = crate::Result<()>> + Send + 'static,
{
    SseEndpoint::new(handle)
}

/// Upgrades a request to SSE.  This allows you to check beforehand if a request
/// should be upgraded to SSE, instead of [`endpoint`], which directly upgrades
/// the connection.
///
/// # Examples
/// ```rust,no_run
/// # use under::*;
/// use under::sse::Sender;
///
/// async fn sse(request: Request, mut sender: Sender) -> Result<(), anyhow::Error> {
///     sender.send(None, "hello, world!", None).await?;
///     Ok(())
/// }
///
/// fn should_upgrade_to_sse(request: &Request) -> bool {
/// #    return true;
///     // ...
/// }
///
/// async fn handle(request: Request) -> Result<Response, anyhow::Error> {
///     if should_upgrade_to_sse(&request) {
///         under::sse::upgrade(request, sse)
///     } else {
///        Ok(Response::empty_404())
///     }
/// }
///
/// let mut http = under::http();
/// http.at("/sse").get(handle);
/// ```
pub fn upgrade<F, Fut>(request: Request, handle: F) -> Result<Response, anyhow::Error>
where
    F: Fn(Request, Sender) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = crate::Result<()>> + Send + 'static,
{
    handle_sse(request, handle)
}

#[derive(Debug, Clone)]
/// An instance of an SSE endpoint.
///
/// This is created by [`endpoint`], and implements the [`crate::Endpoint`]
/// trait.
pub struct SseEndpoint<F>(Arc<F>);

impl<F> SseEndpoint<F> {
    fn new(f: F) -> Self {
        SseEndpoint(Arc::new(f))
    }
}

#[async_trait]
impl<F, Fut> crate::Endpoint for SseEndpoint<F>
where
    F: Fn(Request, Sender) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = crate::Result<()>> + Send + 'static,
{
    async fn apply(self: Pin<&Self>, request: Request) -> Result<Response, anyhow::Error> {
        let h = self.0.clone();
        // we need this for lifetime extension.  If we pass in `h` directly,
        // `h` would be bound to the lifetime of this function.
        #[allow(clippy::redundant_closure)]
        handle_sse(request, move |r, s| h(r, s))
    }

    fn describe(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SseEndpoint")
            .field(&std::any::type_name::<F>())
            .finish()
    }
}

fn handle_sse<F, Fut>(request: Request, handle: F) -> crate::Result
where
    F: Fn(Request, Sender) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = crate::Result<()>> + Send + 'static,
{
    let (sender, encoder) = async_sse::encode();

    let stream = tokio_util::io::ReaderStream::new(encoder.compat());
    let response = Response::empty_200()
        .with_header("Cache-Control", "no-cache")?
        .with_header("Content-Type", "text/event-stream")?
        .with_body(hyper::Body::wrap_stream(stream));

    tokio::task::spawn(async move {
        if let Err(err) = handle(request, sender).await {
            log::error!("Error handling SSE: {:?}", err);
        }
    });

    Ok(response)
}
