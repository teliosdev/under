use super::Router;
use crate::Endpoint;
use crate::UnderError;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;

impl Router {
    /// Creates a listen server on the specified address.
    ///
    /// The server will prepare the routes, and then start listening for
    /// incoming connections.
    /// # Errors
    /// This can fail if the socket address is invalid, or if the socket is
    /// already in use.
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use under::*;
    /// # #[tokio::main] async fn main() -> Result<(), anyhow::Error> {
    /// let mut http = under::http();
    /// http.at("/").get(|_| async { Response::text("hello, world!") });
    /// http.listen("0.0.0.0:8080").await?;
    /// # Ok(())
    /// # }
    /// ```
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

        let termination = self.terminate.take();
        let termination = async {
            match termination {
                Some(mut tx) => loop {
                    if *tx.borrow() {
                        break;
                    }
                    match tx.changed().await {
                        Ok(_) => continue,
                        Err(_) => futures::future::pending().await,
                    }
                },
                None => futures::future::pending().await,
            }
        };

        let this = Arc::pin(self);

        hyper::server::Server::bind(&address)
            .serve(hyper::service::make_service_fn(
                |v: &hyper::server::conn::AddrStream| {
                    let router = this.clone();
                    let service = RouterService(router, v.remote_addr());
                    async move { Ok::<_, std::convert::Infallible>(service) }
                },
            ))
            .with_graceful_shutdown(termination)
            .await
            .map_err(UnderError::HyperServer)?;

        Ok(())
    }
}

#[derive(Clone)]
struct RouterService(Pin<Arc<Router>>, std::net::SocketAddr);

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

    fn call(&mut self, mut request: hyper::Request<hyper::Body>) -> Self::Future {
        let this = (self.0).clone();
        let addr = crate::middleware::PeerAddress(self.1);
        request.extensions_mut().insert(addr);
        Box::pin(async move { this.as_ref().apply(request.into()).await.map(Into::into) })
    }
}
