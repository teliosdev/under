use crate::request::Request;
use crate::response::IntoResponse;
use crate::Response;
use std::future::Future;
use std::pin::Pin;

#[async_trait]
/// An HTTP request handler.
///
/// This is automatically implemented for
/// `Fn(Request) -> impl Future<Output = impl IntoResponse>` types, but it may
/// be useful to implement this yourself.  All this is meant to do is be a
/// fallible function from a [`Request`] into a [`Response`].
pub trait Endpoint: Send + Sync + 'static {
    #[must_use]
    /// Transforms the request into the response.  However, a request may fail,
    /// and such a failure can be handled by down the stack.
    async fn apply(self: Pin<&Self>, request: Request) -> Result<Response, anyhow::Error>;

    #[doc(hidden)]
    fn describe(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", std::any::type_name::<Self>())
    }
}

impl std::fmt::Debug for dyn Endpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.describe(f)
    }
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
