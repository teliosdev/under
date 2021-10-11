use std::pin::Pin;

use super::Endpoint;
use crate::request::Request;
use crate::response::{IntoResponse, Response};
use anyhow::Error;

pub(super) struct SyncEndpoint<F>(pub(super) F);

#[async_trait]
impl<F, Res> Endpoint for SyncEndpoint<F>
where
    F: Fn(Request) -> Res + Send + Sync + 'static,
    Res: IntoResponse + Send + 'static,
{
    async fn apply(self: Pin<&Self>, request: Request) -> Result<Response, Error> {
        let f = &self.0;
        f(request).into_response()
    }
}
