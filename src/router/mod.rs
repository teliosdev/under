use crate::endpoint::Endpoint;
use std::fmt::Write;
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
pub struct Router<D> {
    regex: regex::RegexSet,
    pub(crate) routes: Vec<Arc<Route<D>>>,
}

impl<D> Router<D> {
    pub fn new() -> Self {
        Router {
            regex: regex::RegexSet::new::<Option<&str>, _>(None).unwrap(),
            routes: vec![],
        }
    }

    #[doc(hidden)]
    pub fn compile(&mut self) {
        let patterns = self.routes.iter().map(|route| route.regex.as_str());
        let set = regex::RegexSet::new(patterns).unwrap();
        self.regex = set;
    }

    pub fn at<P: AsRef<str>>(&mut self, prefix: P) -> RoutePath<'_, D> {
        RoutePath {
            prefix: join_paths("", prefix.as_ref()),
            builder: &mut self.routes,
        }
    }

    pub(crate) fn lookup(&self, path: &str, method: &hyper::Method) -> Option<Arc<Route<D>>> {
        self.regex
            .matches(path)
            .into_iter()
            .map(|i| &self.routes[i])
            .filter(|r| r.method.is_none() || r.method.as_ref() == Some(method))
            .next_back()
            .cloned()
    }
}

impl<D> std::fmt::Debug for Router<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Router")
            .field("regex", &self.regex)
            .field("routes", &self.routes)
            .finish()
    }
}

pub(crate) struct Route<D> {
    pub(crate) path: String,
    pub(crate) regex: regex::Regex,
    pub(crate) method: Option<hyper::Method>,
    pub(crate) endpoint: Box<dyn Endpoint<D>>,
}

impl<D> std::fmt::Debug for Route<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Route")
            .field("path", &self.path)
            .field("method", &self.method)
            .finish()
    }
}

/// A description of a path in the router.  This is generated when you call [`Stack::at`], and it
/// contains the passed prefix from that function.  Here, you can specify the behavior to perform
/// at that prefix - the [`Endpoint`]s to perform on each method of that Path.
///
/// [`Stack::at`]: struct.Stack.html#method.at
/// [`Endpoint`]: trait.Endpoint.html
pub struct RoutePath<'a, D> {
    prefix: String,
    builder: &'a mut Vec<Arc<Route<D>>>,
}

macro_rules! method {
    ($($(#[$m:meta])* $v:vis fn $n:ident = $meth:expr;)+) => {
        $(
            $(#[$m])* $v fn $n<E: Endpoint<D>>(&mut self, endpoint: E) -> &mut Self {
                self.method($meth, endpoint)
            }
        )+
    };
}

impl<'a, D> RoutePath<'a, D> {
    /// This appends to the prefix, creating a new [`RoutePath`] from the current one and the given
    /// supplemental prefix.  This assumes that the prefix is never terminated with a forward
    /// slash, but always prefixed with one.
    ///
    /// # Example
    /// ```rust
    /// # fn main() {
    /// # use short::{Stack, Response, endpoint::static_endpoint};
    /// # let mut router = Stack::new();
    /// # let user_index = static_endpoint(Response::empty_204);
    /// # let user_show = static_endpoint(Response::empty_204);
    /// # let user_update = static_endpoint(Response::empty_204);
    /// # let user_destroy = static_endpoint(Response::empty_204);
    /// let mut base = router.at("/user");
    /// base.get(user_index);
    /// base.at("/{id}")
    ///     .get(user_show)
    ///     .post(user_update)
    ///     .delete(user_destroy);
    /// # router.compile();
    /// # }
    /// ```
    ///
    /// [`RoutePath`]: struct.RoutePath.html
    pub fn at<P: AsRef<str>>(&mut self, path: P) -> RoutePath<'_, D> {
        RoutePath {
            prefix: join_paths(&self.prefix, path.as_ref()),
            builder: self.builder,
        }
    }

    pub fn all<E: Endpoint<D>>(&mut self, endpoint: E) -> &mut Self {
        self.builder.push(Arc::new(Route {
            path: self.prefix.clone(),
            regex: regex::Regex::new(&regex_pattern(&self.prefix)).unwrap(),
            method: None,
            endpoint: Box::new(endpoint),
        }));
        self
    }

    pub fn method<E: Endpoint<D>>(&mut self, method: hyper::Method, endpoint: E) -> &mut Self {
        self.builder.push(Arc::new(Route {
            path: self.prefix.clone(),
            regex: regex::Regex::new(&regex_pattern(&self.prefix)).unwrap(),
            method: Some(method),
            endpoint: Box::new(endpoint),
        }));
        self
    }

    method![
        /// Creates a GET endpoint at the current prefix.
        ///
        /// # Example
        /// ```rust
        /// # #[tokio::main]
        /// # async fn main() {
        /// # use short::endpoint::static_endpoint;
        /// # let mut router = short::Stack::new();
        /// # let endpoint = static_endpoint(short::Response::empty_204);
        /// router.at("/user").get(endpoint);
        /// router.compile();
        /// let response = router.response_for("/user", &hyper::Method::GET).await.unwrap();
        /// # assert_eq!(response.unwrap().status(), hyper::StatusCode::NO_CONTENT);
        /// # }
        /// ```
        pub fn get = hyper::Method::GET;
        /// TODO.
        pub fn post = hyper::Method::POST;
        /// TODO.
        pub fn put = hyper::Method::PUT;
        /// TODO.
        pub fn delete = hyper::Method::DELETE;
        /// TODO.
        pub fn head = hyper::Method::HEAD;
        /// TODO.
        pub fn trace = hyper::Method::TRACE;
        /// TODO.
        pub fn connect = hyper::Method::CONNECT;
        /// TODO.
        pub fn patch = hyper::Method::PATCH;
    ];
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

lazy_static::lazy_static! {
    static ref PATTERN: regex::Regex = regex::Regex::new("\\{(?P<name>[a-zA-Z]+)?(?::(?P<pattern>[a-zA-Z]+))?\\}").unwrap();
}

fn regex_pattern(path: &str) -> String {
    let mut start = 0;
    let mut buffer = String::with_capacity(path.len() + 2);
    buffer.push('^');

    for matches in PATTERN.find_iter(path) {
        buffer.push_str(&regex::escape(&path[start..matches.start()]));
        start = matches.end();
        let capture = PATTERN.captures(matches.as_str()).unwrap();
        let name = capture.name("name").map(|m| m.as_str());
        let pattern = capture.name("pattern").map(|m| m.as_str());
        push_pattern(&mut buffer, name, pattern);
    }

    buffer.push_str(&regex::escape(&path[start..]));

    buffer.push('$');
    buffer
}

static UUID_PATTERN: &str =
    "[a-fA-F0-9]{8}-[a-fA-F0-9]{4}-4[a-fA-F0-9]{3}-[89aAbB][a-fA-F0-9]{3}-[a-fA-F0-9]{12}";

fn push_pattern(buffer: &mut String, name: Option<&str>, pattern: Option<&str>) {
    struct NamePattern<'n>(Option<&'n str>);
    impl std::fmt::Display for NamePattern<'_> {
        fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            if let Some(n) = self.0 {
                write!(fmt, "?P<{}>", n)
            } else {
                Ok(())
            }
        }
    }
    let name = NamePattern(name);
    match pattern {
        Some("oext") => write!(buffer, "(?:\\.({}[^/]+))?", name),
        Some("int") => write!(buffer, "({}[+-]?\\d+)", name),
        Some("uint") => write!(buffer, "({}\\d+)", name),
        Some("path") => write!(buffer, "({}.+)", name),
        Some("uuid") => write!(buffer, "({}{})", name, UUID_PATTERN),
        Some("str") | Some("s") | Some("string") | None => write!(buffer, "({}[^/]+)", name),
        Some(v) => panic!("unknown path pattern type {:?}", v),
    }
    .unwrap();
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::request::Request;
    use crate::response::Response;

    async fn simple_endpoint(_: Request<()>) -> Result<Response, anyhow::Error> {
        unimplemented!()
    }

    fn simple_router() -> Router<()> {
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
        let result = router.lookup("/", &hyper::Method::GET);
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!("/", &result.path);
    }

    #[test]
    fn test_simple_match() {
        let router = simple_router();
        let result = router.lookup("/beta/4444", &hyper::Method::GET);
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!("/beta/{id}", &result.path);
    }

    #[test]
    fn test_multi_match() {
        let router = simple_router();
        let result = router.lookup("/gamma/a/b/c", &hyper::Method::GET);
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!("/gamma/{all:path}", &result.path);
    }

    #[test]
    fn test_missing_match() {
        let router = simple_router();
        let result = router.lookup("/omega/aaa", &hyper::Method::GET);
        assert!(result.is_none());
    }

    #[test]
    fn test_correct_method() {
        let router = simple_router();
        let result = router.lookup("/alpha", &hyper::Method::POST);
        assert!(result.is_none());
    }
}
