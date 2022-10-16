use super::Pattern;
use crate::Endpoint;
use std::pin::Pin;
use std::sync::Arc;

pub(crate) struct Route {
    pub(crate) path: String,
    pub(crate) pattern: Pattern,
    method: Option<http::Method>,
    endpoint: Pin<Box<dyn Endpoint>>,
}

impl Route {
    /// Get a reference to the route's method.
    pub(crate) fn method(&self) -> Option<&http::Method> {
        self.method.as_ref()
    }

    /// Get a reference to the route's endpoint.
    pub(crate) fn endpoint(&self) -> &Pin<Box<dyn Endpoint>> {
        &self.endpoint
    }

    pub(crate) fn matches(&self, method: &http::Method) -> bool {
        self.method.is_none() || self.method.as_ref() == Some(method)
    }
}

impl std::fmt::Debug for Route {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Route")
            .field("path", &self.path)
            .field("method", &self.method)
            .field("endpoint", &self.endpoint)
            .finish_non_exhaustive()
    }
}

/// A description of a path in the router.
///
/// This is generated when you call [`crate::Router::at`], and it contains the
/// passed prefix from that function.  Here, you can specify the behavior to
/// perform at that prefix - the [`Endpoint`]s to perform on each method of
/// that Path.
///
/// Paths are specified to have a specific format.  At any point in the path,
/// you can use a `{}` pattern to denote a fragment.  These fragments can then
/// be accessed in the endpoint (or any middleware) using
/// [`crate::Request::fragment`].  A fragment should have this pattern:
///
/// ```text
/// {[name][:<type>]}
/// ```
///
/// Where `[name]` is the (optional) text-based name for the fragment, and
/// `<type>` is the (optional) type of the fragment (defaulting to string).
/// There are currently six fragment types:
///
/// - `oext`: matches an (optional) extension; e.g. `.jpeg`.  This can be used
///   to allow the front-end to optionally specify the expected content-type
///   of the response (in addition to the `Accept` header).
/// - `int`: matches an integer.  This integer can be positive or negative, and
///   has no bound on length; so there is no guarentee it will fit in any
///   native number sizes.
/// - `uint`: matches an unsigned integer.  This integer _must_ be positive.
///   It similarly has no bound on length.
/// - `path`: matches anything, including path segments (`/`).  This is similar
///   to the `**` glob.
/// - `uuid`: matches an [RFC 4122] UUID.
/// - none / `str` / `s` / `string`: matches any characters excluding a path
///   segment (`/`).
///
/// Note that using an invalid type will currently cause it to panic.  Non-named
/// fragments (e.g. `{}`) must be indexed using numbers, 1-indexed.
///
/// [RFC 4122]: https://datatracker.ietf.org/doc/html/rfc4122
///
/// # Examples
/// ```rust,no_run
/// # use under::*;
/// # async fn expect_response(http: &under::Router, path: &str, status: http::StatusCode) -> Result<(), anyhow::Error> {
/// #     let response = http.handle(Request::get(path)?).await?;
/// #     eprintln!("{}: {} (expected: {})", path, response.status(), status);
/// #     assert_eq!(response.status(), status);
/// #     Ok(())
/// # }
///
/// # #[tokio::main] async fn main() -> Result<(), anyhow::Error> {
/// let endpoint = || under::endpoints::simple(Response::empty_204); // dummy endpoint
/// let mut http = under::http(); // this provides us with the Router instance.
/// http.at("/") // this is the Path instance.
///     .get(endpoint());
///  // specifies a path that should be a `/users/` followed by any
///  // (unsigned) integer, followed by an optional extension (`.json`).
///  http.at("/users/{id:uint}{ext:oext}")
///     .get(endpoint())
///     .post(endpoint());
///  // specifies a path that should start with `/public/`, and then
///  // some text.  This is required for `dir` to work properly.
///  http.at("/public/{:path}")
///     .get(endpoint());
///  // another example.
///  http.at("/actions/{id:uuid}")
///     .get(endpoint());
/// http.prepare();
///
/// use http::StatusCode;
/// eprintln!("{:#?}", http);
/// expect_response(&http, "/users/1", StatusCode::NO_CONTENT).await?;
/// expect_response(&http, "/users/1.json", StatusCode::NO_CONTENT).await?;
/// expect_response(&http, "/users/aaa", StatusCode::INTERNAL_SERVER_ERROR).await?;
/// expect_response(&http, "/public/aa/a", StatusCode::NO_CONTENT).await?;
/// expect_response(&http, "/public/", StatusCode::INTERNAL_SERVER_ERROR).await?;
/// expect_response(&http, "/actions/00000000-0000-0000-0000-000000000000", StatusCode::NO_CONTENT).await?;
/// expect_response(&http, "/actions/1", StatusCode::INTERNAL_SERVER_ERROR).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct Path<'a> {
    pub(super) prefix: String,
    pub(super) builder: &'a mut Vec<Arc<Route>>,
    pub(super) pattern: Option<Pattern>,
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

impl<'a> Path<'a> {
    pub(super) fn new(prefix: impl Into<String>, builder: &'a mut Vec<Arc<Route>>) -> Self {
        Path {
            prefix: prefix.into(),
            builder,
            pattern: None,
        }
    }

    /// This appends to the prefix, creating a new [`Path`] from the
    /// current one and the given supplemental prefix.  This assumes that the
    /// prefix is never terminated with a forward slash, but always prefixed
    /// with one.
    ///
    /// # Example
    /// ```rust
    /// # fn main() {
    /// # use under::{Router, Response, endpoints::simple};
    /// # let mut http = under::http();
    /// # let user_index = simple(Response::empty_204);
    /// # let user_show = simple(Response::empty_204);
    /// # let user_update = simple(Response::empty_204);
    /// # let user_destroy = simple(Response::empty_204);
    /// let mut base = http.at("/user");
    /// base.get(user_index);
    /// base.at("/{id}")
    ///     .get(user_show)
    ///     .post(user_update)
    ///     .delete(user_destroy);
    /// # http.prepare();
    /// # }
    /// ```
    pub fn at<P: AsRef<str>>(&mut self, path: P) -> Path<'_> {
        Path::new(super::join_paths(&self.prefix, path.as_ref()), self.builder)
    }

    /// This appends to the prefix, creating a new [`Path`] from the
    /// current one and the given supplemental prefix.  This assumes that the
    /// prefix is never terminated with a forward slash, but always prefixed
    /// with one.
    ///
    /// The created [`Path`] is then yielded to the given closure, which can
    /// be used to add routes to it; the current [`Path`] is then returned.
    ///
    /// # Example
    /// ```rust
    /// # fn main() {
    /// # use under::{Router, Response, endpoints::simple};
    /// # let mut http = under::http();
    /// # let user_index = simple(Response::empty_204);
    /// # let user_show = simple(Response::empty_204);
    /// # let user_update = simple(Response::empty_204);
    /// # let user_destroy = simple(Response::empty_204);
    /// http.under("/user", |base| {
    ///     base.get(user_index)
    ///         .under("/{id}", |user| {
    ///             user
    ///                 .get(user_show)
    ///                 .post(user_update)
    ///                 .delete(user_destroy);
    ///         });
    /// });
    /// # http.prepare();
    /// # }
    /// ```
    pub fn under<P: AsRef<str>, F: FnOnce(&mut Path<'_>)>(&mut self, path: P, f: F) -> &mut Self {
        let mut base = self.at(path);
        f(&mut base);
        self
    }

    /// Creates an endpoint responding to any method at the current prefix.
    ///
    /// # Examples
    /// ```rust
    /// # #[tokio::main] async fn main() -> Result<(), anyhow::Error> {
    /// # use under::*;
    /// # let mut http = under::http();
    /// let endpoint = under::endpoints::simple(Response::empty_204);
    /// let method = http::Method::from_bytes(b"TEST")?;
    /// http.at("/user").all(endpoint);
    /// http.prepare();
    /// let response = http.handle(Request::from_method("/user", method.clone())?).await?;
    /// assert_eq!(response.status(), http::StatusCode::NO_CONTENT);
    /// let response = http.handle(Request::post("/user")?).await?;
    /// assert_eq!(response.status(), http::StatusCode::NO_CONTENT);
    /// # Ok(())
    /// # }
    /// ```
    pub fn all<E: Endpoint>(&mut self, endpoint: E) -> &mut Self {
        let pattern = self.create_pattern();
        self.builder.push(Arc::new(Route {
            path: self.prefix.clone(),
            pattern,
            method: None,
            endpoint: Box::pin(endpoint),
        }));
        self
    }

    /// Creates an endpoint of the specified method at the current prefix.
    ///
    /// # Examples
    /// ```rust
    /// # #[tokio::main] async fn main() -> Result<(), anyhow::Error> {
    /// # use under::*;
    /// # let mut http = under::http();
    /// # let endpoint = under::endpoints::simple(under::Response::empty_204);
    /// let method = http::Method::from_bytes(b"TEST")?;
    /// http.at("/user").method(method.clone(), endpoint);
    /// http.prepare();
    /// let response = http.handle(Request::from_method("/user", method)?).await?;
    /// assert_eq!(response.status(), http::StatusCode::NO_CONTENT);
    /// # Ok(())
    /// # }
    /// ```
    pub fn method<E: Endpoint>(&mut self, method: http::Method, endpoint: E) -> &mut Self {
        let pattern = self.create_pattern();

        self.builder.push(Arc::new(Route {
            path: self.prefix.clone(),
            pattern,
            method: Some(method),
            endpoint: Box::pin(endpoint),
        }));
        self
    }

    method![
        /// Creates a GET endpoint at the current prefix.
        ///
        /// # Examples
        /// ```rust
        /// # #[tokio::main] async fn main() -> Result<(), anyhow::Error> {
        /// # use under::*;
        /// let mut http = under::http();
        /// let endpoint = under::endpoints::simple(under::Response::empty_204);
        /// http.at("/user").get(endpoint);
        /// http.prepare();
        /// let response = http.handle(under::Request::get("/user")?).await?;
        /// # assert_eq!(response.status(), http::StatusCode::NO_CONTENT);
        /// # Ok(())
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

    fn create_pattern(&mut self) -> Pattern {
        if let Some(pattern) = self.pattern.clone() {
            pattern
        } else {
            let pattern = Pattern::new(&self.prefix);
            self.pattern = Some(pattern.clone());
            pattern
        }
    }
}
