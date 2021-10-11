mod trace;
pub use self::trace::TraceMiddleware;
use crate::{Endpoint, Request, Response};
use std::pin::Pin;

pub struct Next<'a> {
    pub(crate) middleware: &'a [Pin<Box<dyn Middleware>>],
    pub(crate) endpoint: Pin<&'a dyn Endpoint>,
}

#[async_trait]
#[doc(notable_trait)]
pub trait Middleware: Send + Sync + 'static {
    async fn apply(
        self: Pin<&Self>,
        request: Request,
        next: Next<'_>,
    ) -> Result<Response, anyhow::Error>;
}

impl<'a> Next<'a> {
    pub(crate) fn new(
        middleware: &'a [Pin<Box<dyn Middleware>>],
        endpoint: Pin<&'a dyn Endpoint>,
    ) -> Self {
        Next {
            middleware,
            endpoint,
        }
    }

    pub async fn apply(self, request: Request) -> Result<Response, anyhow::Error> {
        if let Some((current, next)) = self.middleware.split_first() {
            let new = Next {
                middleware: next,
                endpoint: self.endpoint,
            };
            current.as_ref().apply(request, new).await
        } else {
            self.endpoint.apply(request).await
        }
    }
}
