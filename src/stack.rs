use crate::endpoint::Endpoint;
use crate::middleware::Middleware;
use crate::router::Router;
use crate::{Response, ShortError};
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;

pub struct Stack {
    routes: Router,
    middleware: Vec<Pin<Box<dyn Middleware>>>,
    default: Pin<Box<dyn Endpoint>>,
}

impl Stack {
    pub fn new() -> Self {
        Stack {
            routes: Router::new(),
            middleware: vec![],
            default: Box::pin(default_endpoint()),
        }
    }

    pub fn at<P: AsRef<str>>(&mut self, prefix: P) -> super::router::RoutePath<'_> {
        self.routes.at(prefix)
    }

    #[doc(hidden)]
    pub fn routes_mut(&mut self) -> &mut Router {
        &mut self.routes
    }

    pub fn with<M: Middleware>(&mut self, middleware: M) -> &mut Self {
        self.middleware.push(Box::pin(middleware));
        self
    }

    pub async fn listen(mut self, address: &str) -> Result<(), ShortError> {
        let address: SocketAddr = address
            .parse()
            .map_err(|_| ShortError::InvalidAddress(address.to_owned()))?;
        self.compile();

        log::info!("listen({})", address);

        if log::log_enabled!(log::Level::Trace) {
            for route in self.routes.routes.iter() {
                log::trace!(
                    "route: {} {} ({:?})",
                    route.method.as_ref().map(|m| m.as_str()).unwrap_or("(all)"),
                    route.path,
                    route.regex
                );
            }
        }

        let this = StackService(Arc::new(self));

        hyper::server::Server::bind(&address)
            .serve(hyper::service::make_service_fn(|_| {
                let stack = this.clone();
                async move { Ok::<_, std::convert::Infallible>(stack) }
            }))
            .await
            .map_err(ShortError::HyperServeError)?;

        Ok(())
    }

    pub fn compile(&mut self) {
        self.routes.compile();
    }

    async fn handle(
        self: Arc<Self>,
        request: http::Request<hyper::Body>,
    ) -> Result<http::Response<hyper::Body>, anyhow::Error> {
        self.request(request).await
    }

    pub async fn request(
        &self,
        request: http::Request<hyper::Body>,
    ) -> Result<http::Response<hyper::Body>, anyhow::Error> {
        let route = self.routes.lookup(request.uri().path(), request.method());
        let request: crate::Request = request.into();

        log::trace!("request.route: {:?}", route);

        let endpoint = route
            .as_ref()
            .or(self.routes.default.as_ref())
            .map(|e| e.endpoint.as_ref())
            .unwrap_or(self.default.as_ref());

        let next = crate::middleware::Next::new(&self.middleware[..], endpoint);

        match next.apply(request).await {
            Ok(v) => Ok(v.into_inner()),
            Err(e) => {
                log::error!("request.error: {}", e);
                log::trace!("request.error.debug: {:?}", e);
                Ok(crate::Response::empty_500().into_inner())
            }
        }
    }
}

fn default_endpoint() -> impl Endpoint + 'static {
    crate::endpoint::r#static(|| Response::empty_500())
}

#[derive(Clone)]
struct StackService(Arc<Stack>);

impl tower::Service<hyper::Request<hyper::Body>> for StackService {
    type Response = hyper::Response<hyper::Body>;

    type Error = anyhow::Error;

    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: hyper::Request<hyper::Body>) -> Self::Future {
        Box::pin((self.0).clone().handle(req))
    }
}
