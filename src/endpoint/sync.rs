use super::Endpoint;
use crate::request::Request;
use crate::response::Response;
use anyhow::Error;
use std::future::Future;
use std::pin::Pin;

pub(super) struct SyncEndpoint<F>(pub(super) F);

impl<D, F, Er> Endpoint<D> for SyncEndpoint<F>
where
    D: Send + Sync + 'static,
    F: for<'a> Fn(Request<D>) -> Result<Response, Er> + Send + Sync + 'static,
    Er: Into<anyhow::Error>,
{
    fn apply<'s, 'a>(
        &'s self,
        request: Request<D>,
    ) -> Pin<Box<dyn Future<Output = Result<Response, Error>> + Send + 'a>>
    where
        's: 'a,
        Self: 'a,
    {
        Box::pin(async move {
            let f = &self.0;
            f(request).map_err(|e| e.into())
        })
    }
}
