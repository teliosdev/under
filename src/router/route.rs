use crate::Endpoint;
use std::fmt::Write;
use std::pin::Pin;
use std::sync::Arc;

pub(crate) struct Route {
    pub(crate) path: String,
    pub(crate) regex: regex::Regex,
    pub(crate) method: Option<http::Method>,
    pub(crate) endpoint: Pin<Box<dyn Endpoint>>,
}

impl std::fmt::Debug for Route {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Route")
            .field("path", &self.path)
            .field("method", &self.method)
            .finish()
    }
}

/// A description of a path in the router.  This is generated when you call
/// [`Stack::at`], and it contains the passed prefix from that function.  Here,
/// you can specify the behavior to perform at that prefix - the [`Endpoint`]s
/// to perform on each method of that Path.
///
/// [`Stack::at`]: struct.Stack.html#method.at
/// [`Endpoint`]: trait.Endpoint.html
pub struct RoutePath<'a> {
    pub(super) prefix: String,
    pub(super) builder: &'a mut Vec<Arc<Route>>,
}

macro_rules! method {
    ($($(#[$m:meta])* $v:vis fn $n:ident = $meth:expr;)+) => {
        $(
            $(#[$m])* $v fn $n<E: Endpoint>(&mut self, endpoint: E) -> &mut Self {
                self.method($meth, endpoint)
            }
        )+
    };
}

impl<'a> RoutePath<'a> {
    /// This appends to the prefix, creating a new [`RoutePath`] from the
    /// current one and the given supplemental prefix.  This assumes that the
    /// prefix is never terminated with a forward slash, but always prefixed
    /// with one.
    ///
    /// # Example
    /// ```rust
    /// # fn main() {
    /// # use under::{Stack, Response, endpoint::r#static};
    /// # let mut http = Stack::new();
    /// # let user_index = r#static(Response::empty_204);
    /// # let user_show = r#static(Response::empty_204);
    /// # let user_update = r#static(Response::empty_204);
    /// # let user_destroy = r#static(Response::empty_204);
    /// let mut base = http.at("/user");
    /// base.get(user_index);
    /// base.at("/{id}")
    ///     .get(user_show)
    ///     .post(user_update)
    ///     .delete(user_destroy);
    /// # http.compile();
    /// # }
    /// ```
    ///
    /// [`RoutePath`]: struct.RoutePath.html
    pub fn at<P: AsRef<str>>(&mut self, path: P) -> RoutePath<'_> {
        RoutePath {
            prefix: super::join_paths(&self.prefix, path.as_ref()),
            builder: self.builder,
        }
    }

    pub fn all<E: Endpoint>(&mut self, endpoint: E) -> &mut Self {
        self.builder.push(Arc::new(Route {
            path: self.prefix.clone(),
            regex: regex::Regex::new(&regex_pattern(&self.prefix)).unwrap(),
            method: None,
            endpoint: Box::pin(endpoint),
        }));
        self
    }

    pub fn method<E: Endpoint>(&mut self, method: http::Method, endpoint: E) -> &mut Self {
        self.builder.push(Arc::new(Route {
            path: self.prefix.clone(),
            regex: regex::Regex::new(&regex_pattern(&self.prefix)).unwrap(),
            method: Some(method),
            endpoint: Box::pin(endpoint),
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
        /// # use under::endpoint::r#static;
        /// # let mut http = under::http();
        /// # let endpoint = r#static(under::Response::empty_204);
        /// http.at("/user").get(endpoint);
        /// http.compile();
        /// let response = http.request("/user", &http::Method::GET).await.unwrap();
        /// # assert_eq!(response.unwrap().status(), http::StatusCode::NO_CONTENT);
        /// # }
        /// ```
        pub fn get = http::Method::GET;
        /// TODO.
        pub fn post = http::Method::POST;
        /// TODO.
        pub fn put = http::Method::PUT;
        /// TODO.
        pub fn delete = http::Method::DELETE;
        /// TODO.
        pub fn head = http::Method::HEAD;
        /// TODO.
        pub fn trace = http::Method::TRACE;
        /// TODO.
        pub fn connect = http::Method::CONNECT;
        /// TODO.
        pub fn patch = http::Method::PATCH;
    ];
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
