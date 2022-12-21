mod pattern;
mod route;
mod service;

pub(crate) use self::pattern::Pattern;
pub use self::route::Path;
pub(crate) use self::route::Route;
use crate::endpoint::Endpoint;
use crate::middleware::Middleware;
use crate::{Request, Response};
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::watch;

/// An HTTP router.
///
/// This contains a set of paths, and the [`Endpoint`]s they point
/// to.  This expects a root, `/`, and all paths placed in this router are
/// expected to be based off of this root.  Ultimately, this is an array of
/// routes, where each route is a path, a method, and an endpoint.  If
/// the incoming request matches on the path and method, then the last route
/// inserted that matches will have its endpoint run.  So, assuming that you
/// have the following routes defined:
///
/// ```text
/// // ...
/// POST /user/{id} -> endpoint_user_id
/// POST /user/@me -> endpoint_user_me
/// // ...
/// ```
///
/// Even though the former route can match `/user/@me`, the latter route will
/// always be picked instead, as it is defined _after_ the former.
///
/// # Internals
///
/// Internally, the router uses a regular expression matcher to convert the
/// given paths (e.g. `/user/{id}`) into a regular expression
/// (`^/user/(?P<id>[^/]+)`).  It does this segment-by-segment in the path, and
/// is rather strict about what the names of a placeholder component can be
/// (only alphabetical).  This is compiled into a `RegexSet`, which, when run
/// against a given path, will return a list of routes that the path matches.
/// Because we don't have to fool around with matching every route, the timing
/// is `O(n)`, with `n` being the length of the input path.  After the
/// `RegexSet` match, we again match against the route to collect the pattern
/// matchers (e.g. `{some}` and `{value:path}`), before returning both.  This
/// information is included as a part of the request.
pub struct Router {
    regex: regex::RegexSet,
    routes: Vec<Arc<Route>>,
    middleware: Vec<Pin<Box<dyn Middleware>>>,
    fallback: Option<Pin<Box<dyn Endpoint>>>,
    terminate: Option<watch::Receiver<bool>>,
}

impl Default for Router {
    fn default() -> Self {
        Router {
            regex: regex::RegexSet::empty(),
            middleware: vec![],
            routes: vec![],
            fallback: None,
            terminate: None,
        }
    }
}

impl Router {
    /// Prepares the router, constructing the routes.
    ///
    /// This is automatically called when listening using [`Router::listen`].
    /// However, you may want to use the router before that point for e.g.
    /// testing, and so this must be called before any requests are routed
    /// (or, if any routes are changed).  If this is not called, you will
    /// receive only 500 errors.
    pub fn prepare(&mut self) {
        let patterns = self
            .routes
            .iter()
            .map(|route| route.pattern.regex().as_str());
        let set = regex::RegexSet::new(patterns).unwrap();
        self.regex = set;
    }

    pub(crate) fn routes(&self) -> &[Arc<Route>] {
        &self.routes[..]
    }

    /// Creates a [`Path`] at the provided prefix.  See [`Path::at`] for more.
    pub fn at<P: AsRef<str>>(&mut self, prefix: P) -> Path<'_> {
        Path::new(join_paths("", prefix.as_ref()), &mut self.routes)
    }

    /// Creates a [`Path`] at the provided prefix, and executes the provided
    /// closure with it.  See [`Path::under`] for more.
    pub fn under<P: AsRef<str>, F: FnOnce(&mut Path<'_>)>(
        &mut self,
        prefix: P,
        build: F,
    ) -> &mut Self {
        let mut path = Path::new(join_paths("", prefix.as_ref()), &mut self.routes);
        build(&mut path);
        self
    }

    /// Appends middleware to the router.  Each middleware is executed in the
    /// order that it is appended to the router (i.e., the first middleware
    /// inserted executes first).
    ///
    /// # Examples
    /// ```rust
    /// let mut http = under::http();
    /// http.with(under::middleware::TraceMiddleware::new())
    ///     .with(under::middleware::StateMiddleware::new(123u32));
    /// ```
    pub fn with<M: Middleware>(&mut self, middleware: M) -> &mut Self {
        self.middleware.push(Box::pin(middleware));
        self
    }

    /// Sets a fallback endpoint.  If there exists no other endpoint in the
    /// router that could potentially respond to the request, it will first
    /// attempt to execute this fallback endpoint, before instead returning
    /// an empty 500 error.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// # #[tokio::main] async fn main() -> Result<(), anyhow::Error> {
    /// let mut http = under::http();
    /// http.at("/foo").get(under::endpoints::simple(Response::empty_204));
    /// http.fallback(under::endpoints::simple(Response::empty_404));
    /// http.prepare();
    /// let response = http.handle(Request::get("/foo")?).await?;
    /// assert_eq!(response.status(), http::StatusCode::NO_CONTENT);
    /// let response = http.handle(Request::get("/bar")?).await?;
    /// assert_eq!(response.status(), http::StatusCode::NOT_FOUND);
    /// # Ok(())
    /// # }
    pub fn fallback<E: Endpoint>(&mut self, endpoint: E) -> &mut Self {
        self.fallback = Some(Box::pin(endpoint));
        self
    }

    /// A channel to handle the termination singal.  By default, the router does
    /// not terminate, at least not gracefully, even in the face of
    /// SIGINT/SIGTERM.  This allows you to signal to the router when it should
    /// terminate, and it will gracefully shut down, letting all current
    /// requests finish before exiting.  Note that the return type is not
    /// `Clone`, and dropping the sender will not terminate the router.
    ///
    /// Note this only applies to the router when listening, and not when
    /// handling a single request.
    pub fn termination_signal(&mut self) -> watch::Sender<bool> {
        let (tx, rx) = watch::channel(false);
        self.terminate = Some(rx);
        tx
    }

    /// Handles a one-off request to the router.  This is equivalent to pinning
    /// the router (with [`Pin::new`], since the Router is `Unpin`), before
    /// calling [`crate::Endpoint::apply`].
    pub async fn handle(&self, request: Request) -> Result<Response, anyhow::Error> {
        Pin::new(self).apply(request).await
    }

    pub(crate) fn lookup(&self, path: &str, method: &http::Method) -> Option<Arc<Route>> {
        self.regex
            .matches(path)
            .into_iter()
            .map(|i| &self.routes[i])
            .filter(|r| r.matches(method))
            .next_back()
            .cloned()
    }

    fn fallback_endpoint(&self) -> Option<Pin<&dyn Endpoint>> {
        self.fallback.as_ref().map(Pin::as_ref)
    }
}

#[async_trait]
impl crate::Endpoint for Router {
    async fn apply(self: Pin<&Self>, mut request: Request) -> Result<Response, anyhow::Error> {
        let route = self.lookup(request.uri().path(), request.method());
        if let Some(route) = route.clone() {
            // This should most always be a `Some`, because the route's path
            // would 100% match the uri's path.
            if let Some(fragment) =
                crate::request::fragment::Fragment::new(request.uri().path(), &*route)
            {
                request.extensions_mut().insert(fragment);
            }
            request.extensions_mut().insert(route);
        }

        let endpoint = {
            let route_endpoint = || route.as_ref().map(|e| e.endpoint().as_ref());
            let fallback_endpoint = || self.fallback_endpoint();
            route_endpoint()
                .or_else(fallback_endpoint)
                .unwrap_or_else(default_endpoint)
        };
        log::trace!("{} {} --> {:?}", request.method(), request.uri(), endpoint);
        let next = crate::middleware::Next::new(&self.middleware[..], endpoint);
        next.apply(request).await
    }
}

impl std::fmt::Debug for Router {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Router")
            .field("regex", &self.regex)
            .field("routes", &self.routes)
            .finish()
    }
}

lazy_static::lazy_static! {
    static ref DEFAULT_ENDPOINT: crate::endpoints::SyncEndpoint<fn(Request) -> Response> = crate::endpoints::SyncEndpoint::new(|_| Response::empty_500());
    static ref DEFAULT_ENDPOINT_PIN: Pin<&'static (dyn Endpoint + Unpin + 'static)> = Pin::new(&*DEFAULT_ENDPOINT);
}

// 'r can be anything _up to and including_ 'static, and this makes it play
// nice with unwrap_or_else.
pub(crate) fn default_endpoint<'r>() -> Pin<&'r dyn Endpoint> {
    *DEFAULT_ENDPOINT_PIN
}

// Base *MUST* be either `""` or start with `"/"`.
fn join_paths(base: &str, extend: &str) -> String {
    let mut buffer = String::with_capacity(base.len() + extend.len());
    buffer.push_str(base);

    match (base.ends_with('/'), extend.starts_with('/')) {
        (true, true) => {
            buffer.push_str(&extend[1..]);
        }
        (false, true) | (true, false) => {
            buffer.push_str(extend);
        }
        (false, false) => {
            buffer.push('/');
            buffer.push_str(extend);
        }
    }

    buffer.shrink_to_fit();
    buffer
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::request::Request;
    use crate::response::Response;
    use crate::UnderError;

    #[allow(clippy::unused_async)]
    async fn simple_endpoint(_: Request) -> Result<Response, UnderError> {
        unimplemented!()
    }

    fn simple_router() -> Router {
        let mut router = Router::default();
        router.at("/").get(simple_endpoint);
        router.at("/alpha").get(simple_endpoint);
        router.at("/beta/{id}").get(simple_endpoint);
        router.at("/gamma/{all:path}").get(simple_endpoint);
        router.prepare();
        router
    }

    #[test]
    fn test_join_paths() {
        assert_eq!(join_paths("", "/id"), "/id");
        assert_eq!(join_paths("", "id"), "/id");
        assert_eq!(join_paths("/user", "/id"), "/user/id");
        assert_eq!(join_paths("/user/", "/id"), "/user/id");
        assert_eq!(join_paths("/user/", "id"), "/user/id");
    }

    #[test]
    fn test_build() {
        simple_router();
    }

    #[test]
    fn test_basic_match() {
        let router = simple_router();
        dbg!(&router);
        let result = router.lookup("/", &http::Method::GET);
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!("/", &result.path);
    }

    #[test]
    fn test_simple_match() {
        let router = simple_router();
        let result = router.lookup("/beta/4444", &http::Method::GET);
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!("/beta/{id}", &result.path);
    }

    #[test]
    fn test_multi_match() {
        let router = simple_router();
        let result = router.lookup("/gamma/a/b/c", &http::Method::GET);
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!("/gamma/{all:path}", &result.path);
    }

    #[test]
    fn test_missing_match() {
        let router = simple_router();
        let result = router.lookup("/omega/aaa", &http::Method::GET);
        assert!(result.is_none());
    }

    #[test]
    fn test_correct_method() {
        let router = simple_router();
        let result = router.lookup("/alpha", &http::Method::POST);
        assert!(result.is_none());
    }
}
