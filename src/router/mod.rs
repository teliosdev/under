mod route;

pub(crate) use self::route::Route;
pub use self::route::RoutePath;
use crate::endpoint::Endpoint;
use std::sync::Arc;

/// A router.  This contains a set of paths, and the [`Endpoint`]s they point to.  This expects
/// a root, `/`, and all paths placed in this router are expected to be based off of this root.
/// Ultimately, this is an array of `Route`s, where each `Route` is a path, a method, and an
/// endpoint.  If the incoming request matches on the path and method, then the last route inserted
/// that matches will have its endpoint run.  So, assuming that you have the following routes
/// defined:
///
/// ```text
/// // ...
/// POST /user/{id} -> endpoint_user_id
/// POST /user/@me -> endpoint_user_me
/// // ...
/// ```
///
/// Even though the former route can match `/user/@me`, the latter route will always be picked
/// instead, as it is defined _after_ the former.
///
/// # Internals
///
/// Internally, the router uses a regular expression matcher to convert the given paths (e.g.
/// `/user/{id}`) into a regular expression (`^/user/(?P<id>[^/]+)`).  It does this
/// segment-by-segment in the path, and is rather strict about what the names of a placeholder
/// component can be (only alphabetical).  This is compiled into a `RegexSet`, which, when run
/// against a given path, will return a list of routes that the path matches.  Because we don't
/// have to fool around with matching every route, the timing is `O(n)`, with `n` being the length
/// of the input path.  After the `RegexSet` match, we again match against the route to collect the
/// pattern matchers (e.g. `{some}` and `{value:path}`), before returning both.  This information is
/// included as a part of the request.
pub struct Router {
    regex: regex::RegexSet,
    pub(crate) routes: Vec<Arc<Route>>,
    pub(crate) default: Option<Arc<Route>>,
}

impl Router {
    pub fn new() -> Self {
        Router {
            regex: regex::RegexSet::new::<Option<&str>, _>(None).unwrap(),
            routes: vec![],
            default: None,
        }
    }

    #[doc(hidden)]
    pub fn compile(&mut self) {
        let patterns = self.routes.iter().map(|route| route.regex.as_str());
        let set = regex::RegexSet::new(patterns).unwrap();
        self.regex = set;
    }

    pub fn at<P: AsRef<str>>(&mut self, prefix: P) -> RoutePath<'_> {
        RoutePath {
            prefix: join_paths("", prefix.as_ref()),
            builder: &mut self.routes,
        }
    }

    pub fn default<E: Endpoint>(&mut self, endpoint: E) {
        self.default = Some(Arc::new(Route {
            path: "/".to_owned(),
            regex: regex::Regex::new("^.*$").unwrap(),
            method: None,
            endpoint: Box::pin(endpoint),
        }));
    }

    pub(crate) fn lookup(&self, path: &str, method: &http::Method) -> Option<Arc<Route>> {
        self.regex
            .matches(path)
            .into_iter()
            .map(|i| &self.routes[i])
            .filter(|r| r.method.is_none() || r.method.as_ref() == Some(method))
            .next_back()
            .or(self.default.as_ref())
            .cloned()
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
    use crate::ShortError;

    async fn simple_endpoint(_: Request) -> Result<Response, ShortError> {
        unimplemented!()
    }

    fn simple_router() -> Router {
        let mut router = Router::new();
        router.at("/").get(simple_endpoint);
        router.at("/alpha").get(simple_endpoint);
        router.at("/beta/{id}").get(simple_endpoint);
        router.at("/gamma/{all:path}").get(simple_endpoint);
        router.compile();
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
        let _ = simple_router();
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
