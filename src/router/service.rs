use crate::Endpoint;
use crate::UnderError;
use std::future::Future;
use std::net::SocketAddr;

use super::*;

impl Router {
    /// # Errors
    pub async fn listen(mut self, address: &str) -> Result<(), UnderError> {
        let address: SocketAddr = address
            .parse()
            .map_err(|_| UnderError::InvalidAddress(address.to_owned()))?;
        self.prepare();

        log::info!("listen({})", address);

        if log::log_enabled!(log::Level::Trace) {
            for route in self.routes() {
                log::trace!(
                    "route: {} {} ({:?})",
                    route.method().map_or("(all)", hyper::Method::as_str),
                    route.path,
                    route.pattern.regex()
                );
            }
        }

        let this = RouterService(Arc::pin(self));

        hyper::server::Server::bind(&address)
            .serve(hyper::service::make_service_fn(|_| {
                let router = this.clone();
                async move { Ok::<_, std::convert::Infallible>(router) }
            }))
            .await
            .map_err(UnderError::HyperServer)?;

        Ok(())
    }
}

#[derive(Clone)]
struct RouterService(Pin<Arc<Router>>);

type RouterFuture<R, E> = Pin<Box<dyn Future<Output = Result<R, E>> + Send + 'static>>;

impl tower::Service<hyper::Request<hyper::Body>> for RouterService {
    type Response = hyper::Response<hyper::Body>;
    type Error = anyhow::Error;
    type Future = RouterFuture<Self::Response, Self::Error>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: hyper::Request<hyper::Body>) -> Self::Future {
        let this = (self.0).clone();
        Box::pin(async move { this.as_ref().apply(request.into()).await.map(Into::into) })
    }
}
