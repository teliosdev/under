pub(crate) mod fragment;

use self::fragment::{Fragment, FragmentSelect};
use std::convert::TryFrom;
use std::str::FromStr;

macro_rules! forward {
    () => {};
    (
        $(#[$m:meta])* $v:vis fn $name:ident(&self $(, $pn:ident: $pt:ty)*) -> $ret:ty;
        $($tail:tt)*
    ) => {
        $(#[$m])* $v fn $name(&self $(, $pn: $pt)*) -> $ret {
            (self.0).$name($($pn),*)
        }

        forward! { $($tail)* }
    };

    (
        $(#[$m:meta])* $v:vis fn $name:ident(&mut self $(, $pn:ident: $pt:ty)*) -> $ret:ty;
        $($tail:tt)*
    ) => {
        $(#[$m])* $v fn $name(&mut self $(, $pn: $pt)*) -> $ret {
            (self.0).$name($($pn),*)
        }

        forward! { $($tail)* }
    }
}

macro_rules! construct {
    () => {};
    ($($(#[$m:meta])* $v:vis fn $method:ident = $action:expr;)+) => {
        $($(#[$m])* $v fn $method<U>(uri: U) -> Result<Self, http::Error>
        where
            http::Uri: TryFrom<U>,
            <http::Uri as TryFrom<U>>::Error: Into<http::Error>
        {
            http::request::Builder::new()
                .method($action)
                .uri(uri)
                .body(hyper::Body::empty())
                .map(Request)
        })+
    };
}

#[derive(Debug)]
/// Represents an HTTP request.
///
/// An HTTP Request consists of a head (a version, a method, a path, and some
/// headers), and a body (which may be empty).  This type offers convenient
/// helpers for constructing HTTP request for you for common use-cases.
///
/// The request also contains an "extensions" type map, which is used by under
/// for containing routing information.  It can also be used to insert state
/// into the request for endpoints.
///
/// # Examples
/// ```rust
/// # use under::*;
/// async fn respond_to(request: Request) -> Result<Response, anyhow::Error> {
///     if request.uri() != "/foo" {
///         return Ok(Response::empty_404());
///     }
///
///     Ok(Response::empty_204())
/// }
/// ```
///
///
pub struct Request(http::Request<hyper::Body>);

impl Request {
    construct! {
        /// Creates a new request initialized with the GET method and the given
        /// URI.
        ///
        /// # Examples
        /// ```rust
        /// # use under::*;
        /// let request = Request::get("https://example.com/a").unwrap();
        /// assert_eq!(request.method(), http::Method::GET);
        /// ```
        pub fn get = http::Method::GET;
        /// Creates a new request initialized with the POST method and the given
        /// URI.
        ///
        /// # Examples
        /// ```rust
        /// # use under::*;
        /// let request = Request::post("https://example.com/a").unwrap();
        /// assert_eq!(request.method(), http::Method::POST);
        /// ```
        pub fn post = http::Method::POST;
        /// Creates a new request initialized with the PUT method and the given
        /// URI.
        ///
        /// # Examples
        /// ```rust
        /// # use under::*;
        /// let request = Request::put("https://example.com/a").unwrap();
        /// assert_eq!(request.method(), http::Method::PUT);
        /// ```
        pub fn put = http::Method::PUT;
        /// Creates a new request initialized with the DELETE method and the given
        /// URI.
        ///
        /// # Examples
        /// ```rust
        /// # use under::*;
        /// let request = Request::delete("https://example.com/a").unwrap();
        /// assert_eq!(request.method(), http::Method::DELETE);
        /// ```
        pub fn delete = http::Method::DELETE;
        /// Creates a new request initialized with the HEAD method and the given
        /// URI.
        ///
        /// # Examples
        /// ```rust
        /// # use under::*;
        /// let request = Request::head("https://example.com/a").unwrap();
        /// assert_eq!(request.method(), http::Method::HEAD);
        /// ```
        pub fn head = http::Method::HEAD;
        /// Creates a new request initialized with the TRACE method and the given
        /// URI.
        ///
        /// # Examples
        /// ```rust
        /// # use under::*;
        /// let request = Request::trace("https://example.com/a").unwrap();
        /// assert_eq!(request.method(), http::Method::TRACE);
        /// ```
        pub fn trace = http::Method::TRACE;
        /// Creates a new request initialized with the CONNECT method and the
        /// given URI.
        ///
        /// # Examples
        /// ```rust
        /// # use under::*;
        /// let request = Request::connect("https://example.com/a").unwrap();
        /// assert_eq!(request.method(), http::Method::CONNECT);
        /// ```
        pub fn connect = http::Method::CONNECT;
        /// Creates a new request initialized with the PATCH method and the
        /// given URI.
        ///
        /// # Examples
        /// ```rust
        /// # use under::*;
        /// let request = Request::patch("https://example.com/a").unwrap();
        /// assert_eq!(request.method(), http::Method::PATCH);
        /// ```
        pub fn patch = http::Method::PATCH;
    }

    /// Creates a new request initialized with the provided method and the
    /// given URI.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// let method = http::Method::from_bytes(b"TEST").unwrap();
    /// let request = Request::from_method("https://example.com/a", method.clone()).unwrap();
    /// assert_eq!(request.method(), method);
    /// ```
    pub fn from_method<U>(uri: U, method: http::Method) -> Result<Self, http::Error>
    where
        http::Uri: TryFrom<U>,
        <http::Uri as TryFrom<U>>::Error: Into<http::Error>,
    {
        http::request::Builder::new()
            .method(method)
            .uri(uri)
            .body(hyper::Body::empty())
            .map(Request)
    }

    /// Retrieves a path fragment from the request, then attempts to parse it.
    /// The key can either be a number, or a string.
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use under::*;
    ///
    /// async fn point(request: Request) -> Response {
    ///     let target: u32 = request.fragment("amount").unwrap();
    ///     Response::text(format!("you bought {} coconuts", target))
    /// }
    ///
    /// # #[tokio::main] async fn main() -> Result<(), anyhow::Error> {
    /// let mut http = under::http();
    /// http.at("/buy/{amount:uint}").get(point);
    /// http.prepare();
    /// let mut response = http.handle(Request::get("/buy/3")?).await?;
    /// assert_eq!(response.status(), http::StatusCode::OK);
    /// let body = response.as_text().await?;
    /// assert_eq!(body, "you bought 3 coconuts");
    /// # Ok(())
    /// # }
    /// ```
    pub fn fragment<I: FromStr, K: FragmentSelect>(&self, key: K) -> Option<I> {
        self.fragment_str(key).and_then(|s| s.parse().ok())
    }

    /// Retrieves a path fragment from the request.  The key can either be
    /// a number, or a string.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    ///
    /// async fn point(request: Request) -> Response {
    ///     let target = request.fragment_str("target").unwrap();
    ///     Response::text(format!("hello, {}", target))
    /// }
    ///
    /// # #[tokio::main] async fn main() -> Result<(), anyhow::Error> {
    /// let mut http = under::http();
    /// http.at("/hello/{target}").get(point);
    /// http.prepare();
    /// let mut response = http.handle(Request::get("/hello/foo")?).await?;
    /// assert_eq!(response.status(), http::StatusCode::OK);
    /// let body = response.as_text().await?;
    /// assert_eq!(body, "hello, foo");
    /// # Ok(())
    /// # }
    /// ```
    pub fn fragment_str<K: FragmentSelect>(&self, key: K) -> Option<&str> {
        self.fragment_ext()?.select(key)
    }

    /// Returns state information provided by the
    /// [`crate::middleware::StateMiddleware`] middleware.  This is a shortcut
    /// to retrieving the [`crate::middleware::State`] extension from the
    /// request.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// use under::middleware::State;
    /// let mut request = Request::get("/").unwrap();
    /// request.extensions_mut().insert(State(123u32));
    /// assert_eq!(request.state::<u32>(), Some(&123u32));
    /// ```
    pub fn state<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.extensions()
            .get::<crate::middleware::State<T>>()
            .map(|v| &v.0)
    }

    fn fragment_ext(&self) -> Option<&Fragment> {
        self.extensions().get::<Fragment>()
    }

    forward! {
        /// Returns a reference to the associated URI.
        ///
        /// # Examples
        /// ```rust
        /// # use under::*;
        /// let request: Request = Request::get("/").unwrap();
        /// assert_eq!(&*request.uri(), "/");
        /// ```
        #[inline]
        pub fn uri(&self) -> &http::Uri;
        /// Returns a reference to the associated HTTP method.
        ///
        /// # Examples
        /// ```rust
        /// # use under::*;
        /// let request: Request = Request::get("/").unwrap();
        /// assert_eq!(*request.method(), http::Method::GET);
        /// ```
        #[inline]
        pub fn method(&self) -> &http::Method;
        /// Returns a reference to the associated header field map.
        ///
        /// # Examples
        ///
        /// ```rust
        /// # use under::*;
        /// let request = Request::get("/").unwrap();
        /// assert!(request.headers().is_empty());
        /// ```
        #[inline]
        pub fn headers(&self) -> &http::HeaderMap<http::HeaderValue>;
        /// Returns a mutable reference to the associated header field map.
        ///
        /// # Examples
        ///
        /// ```
        /// # use under::*;
        /// # use http::header::*;
        /// let mut request = Request::get("/").unwrap();
        /// request.headers_mut().insert(HOST, HeaderValue::from_static("world"));
        /// assert!(!request.headers().is_empty());
        /// ```
        #[inline]
        pub fn headers_mut(&mut self) -> &mut http::HeaderMap<http::HeaderValue>;
        /// Returns a reference to the associated extensions.
        ///
        /// # Examples
        /// ```rust
        /// # use under::*;
        /// let request: Request = Request::get("/").unwrap();
        /// assert!(request.extensions().get::<i32>().is_none());
        /// ```
        #[inline]
        pub fn extensions(&self) -> &http::Extensions;
        /// Returns a mutable reference to the associated extensions.
        ///
        /// # Examples
        /// ```rust
        /// # use under::*;
        /// let mut request: Request = Request::get("/").unwrap();
        /// request.extensions_mut().insert("hello");
        /// assert_eq!(request.extensions().get(), Some(&"hello"));
        /// ```
        #[inline]
        pub fn extensions_mut(&mut self) -> &mut http::Extensions;
    }
}

impl From<http::Request<hyper::Body>> for Request {
    fn from(r: http::Request<hyper::Body>) -> Self {
        Request(r)
    }
}

impl crate::has_body::sealed::Sealed for Request {}
impl crate::has_headers::sealed::Sealed for Request {}

impl crate::HasBody for Request {
    fn body_mut(&mut self) -> &mut hyper::Body {
        self.0.body_mut()
    }

    fn content_type(&self) -> Option<mime::Mime> {
        self.0
            .headers()
            .get(http::header::CONTENT_TYPE)
            .map(|v| v.as_bytes())
            .and_then(|v| std::str::from_utf8(v).ok())
            .and_then(|v| mime::Mime::from_str(v).ok())
    }
}

impl crate::HasHeaders for Request {
    fn headers(&self) -> &http::HeaderMap<http::HeaderValue> {
        self.0.headers()
    }

    fn headers_mut(&mut self) -> &mut http::HeaderMap<http::HeaderValue> {
        self.0.headers_mut()
    }
}
