use crate::error::ShortError;
use crate::router::Router;
use std::convert::Infallible;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

pub struct Stack<D> {
    data: Arc<D>,
    routes: Router<D>,
    default_endpoint: Option<Box<dyn crate::endpoint::Endpoint<D>>>,
}

impl Stack<()> {
    pub fn new() -> Self {
        Stack::with_state(())
    }
}

impl<D: Send + Sync + 'static> Stack<D> {
    pub fn with_state(data: D) -> Stack<D> {
        Stack {
            data: Arc::new(data),
            routes: Router::new(),
            default_endpoint: None,
        }
    }
}

impl<D: Send + Sync + 'static> Stack<D> {
    pub fn at<P: AsRef<str>>(&mut self, prefix: P) -> super::router::RoutePath<'_, D> {
        self.routes.at(prefix)
    }

    #[doc(hidden)]
    pub fn routes_mut(&mut self) -> &mut Router<D> {
        &mut self.routes
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

        hyper::server::Server::bind(&address)
            .serve(MakeArcStack(Arc::new(self)))
            .await
            .map_err(ShortError::HyperServeError)?;
        Ok(())
    }

    pub fn compile(&mut self) {
        self.routes.compile();
    }

    pub async fn response_for(
        &self,
        path: &str,
        method: &hyper::Method,
    ) -> Result<Option<crate::Response>, anyhow::Error> {
        let route = if let Some(r) = self.routes.lookup(path, method) {
            r
        } else {
            return Ok(None);
        };
        let hyper_request = hyper::Request::builder()
            .method(method)
            .uri(path)
            .body(hyper::Body::empty())?;
        let request = crate::Request {
            inner: hyper_request,
            route: Some(route.clone()),
            data: self.data.clone(),
        };
        route.endpoint.apply(request).await.map(Some)
    }

    async fn handle_request(
        self: Arc<Self>,
        request: hyper::Request<hyper::Body>,
    ) -> Result<hyper::Response<hyper::Body>, hyper::Error> {
        let start = std::time::Instant::now();
        let method = request.method().clone();
        let uri = request.uri().clone();

        let route = self.routes.lookup(request.uri().path(), request.method());
        let request = crate::Request {
            inner: request,
            route: route.clone(),
            data: self.data.clone(),
        };

        log::trace!("request.route: {:?}", route);

        let default = default_response();

        let endpoint = route
            .as_ref()
            .map(|e| e.endpoint.as_ref())
            .or(self.default_endpoint.as_deref())
            .unwrap_or(&default);

        match endpoint.apply(request).await {
            Ok(v) => {
                log::info!(
                    "request: {} {} [{}ms] {}",
                    method,
                    uri.path(),
                    start.elapsed().as_millis(),
                    v.status()
                );
                Ok(v.into_inner())
            }
            Err(e) => {
                log::info!(
                    "request: {} {} {}ms {}",
                    method,
                    uri.path(),
                    start.elapsed().as_millis(),
                    hyper::StatusCode::INTERNAL_SERVER_ERROR
                );
                log::error!("request.error: {}", e);
                log::trace!("request.error.debug: {:?}", e);
                Ok(crate::Response::empty_500().into_inner())
            }
        }
    }
}

struct MakeArcStack<D: Send + Sync + 'static>(Arc<Stack<D>>);

struct ArcStack<D: Send + Sync + 'static>(Arc<Stack<D>>);

impl<D: Send + Sync + 'static> hyper::service::Service<hyper::Request<hyper::Body>>
    for ArcStack<D>
{
    type Response = hyper::Response<hyper::Body>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: hyper::Request<hyper::Body>) -> Self::Future {
        let stack = self.0.clone();
        Box::pin(stack.handle_request(req))
    }
}

impl<T, D: Send + Sync + 'static> hyper::service::Service<T> for MakeArcStack<D> {
    type Response = ArcStack<D>;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Infallible>> + Send>>;

    fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _req: T) -> Self::Future {
        let stack = self.0.clone();
        Box::pin(async { Ok(ArcStack(stack)) })
    }
}

fn default_response<D: Send + Sync + 'static>() -> impl crate::Endpoint<D> {
    crate::endpoint::static_endpoint(crate::Response::empty_404)
}
