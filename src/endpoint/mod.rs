use crate::request::Request;
use crate::response::Response;
use std::future::Future;
use std::pin::Pin;

mod dir;
mod sync;

pub trait Endpoint<D>: Send + Sync + 'static {
    #[must_use]
    fn apply<'s, 'a>(
        &'s self,
        request: Request<D>,
    ) -> Pin<Box<dyn Future<Output = Result<Response, anyhow::Error>> + Send + 'a>>
    where
        's: 'a,
        Self: 'a;
}

impl<D, F, Fut, Er> Endpoint<D> for F
where
    D: Send + Sync + 'static,
    F: Fn(Request<D>) -> Fut + Sync + Send + 'static,
    Fut: Future<Output = Result<Response, Er>> + Send + 'static,
    Er: Into<anyhow::Error>,
{
    fn apply<'s, 'a>(
        &'s self,
        request: Request<D>,
    ) -> Pin<Box<dyn Future<Output = Result<Response, anyhow::Error>> + Send + 'a>>
    where
        's: 'a,
        Self: 'a,
    {
        Box::pin(async move { self(request).await.map_err(|e| e.into()) })
    }
}

pub fn sync_endpoint<D, F, Er>(func: F) -> impl Endpoint<D>
where
    D: Send + Sync + 'static,
    F: for<'a> Fn(Request<D>) -> Result<Response, Er> + Send + Sync + 'static,
    Er: Into<anyhow::Error>,
{
    self::sync::SyncEndpoint(func)
}

pub fn ok_sync_endpoint<D, F>(func: F) -> impl Endpoint<D>
where
    D: Send + Sync + 'static,
    F: for<'a> Fn(Request<D>) -> Response + Send + Sync + 'static,
{
    sync_endpoint::<D, _, anyhow::Error>(move |r| Ok(func(r)))
}

pub fn static_endpoint<D, F>(func: F) -> impl Endpoint<D>
where
    D: Send + Sync + 'static,
    F: Fn() -> Response + Send + Sync + 'static,
{
    sync_endpoint::<D, _, anyhow::Error>(move |_| Ok(func()))
}

pub fn dir<P, D>(path: P) -> impl Endpoint<D>
where
    D: Send + Sync + 'static,
    P: Into<std::path::PathBuf>,
{
    self::dir::DirEndpoint::new(path)
}
