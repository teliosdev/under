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

    fn fragment_ext(&self) -> Option<&Fragment> {
        self.extensions().get::<Fragment>()
    }

    /// Parses the query string from the request into the provided type.  If
    /// there is no query string, then `None` is returned; or, if the query
    /// string cannot be parsed into the given type, then `None` is also
    /// returned.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// let request = Request::get("/users?id=1").unwrap();
    /// #[derive(serde::Deserialize)]
    /// struct User { id: u32 }
    /// let user: User = request.query().unwrap();
    /// assert_eq!(user.id, 1);
    /// ```
    pub fn query<'q, S: serde::Deserialize<'q>>(&'q self) -> Option<S> {
        self.uri()
            .query()
            .and_then(|s| serde_qs::from_str::<S>(s).ok())
    }

    /// Attempts to load the peer address of the request.  This is only
    /// available if loaded through the hyper service stack (i.e. the request
    /// originates from [`crate::Router::listen`]), and so cannot garunteed
    /// to be present.
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use under::*;
    ///
    /// async fn handle(request: Request) -> Response {
    ///     let peer = request.peer_addr();
    ///     match request.peer_addr() {
    ///        Some(addr) => Response::text(format!("{}", addr)),
    ///        None => Response::text("no peer address")
    ///     }
    /// }
    ///
    /// # #[tokio::main] async fn main() -> Result<(), anyhow::Error> {
    /// let mut http = under::http();
    /// http.at("/").get(handle);
    /// http.listen("0.0.0.0:8080").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn peer_addr(&self) -> Option<std::net::SocketAddr> {
        Some(self.ext::<crate::middleware::PeerAddress>()?.0)
    }

    /// Sets the peer address of this request to a localhost address.  This is
    /// only useful for testing, and should not be used in production.  This
    /// allows you to test the request handling without having to bind to a
    /// port.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// use std::net::SocketAddr;
    /// let request = Request::get("/").unwrap()
    ///     .with_local_addr();
    /// assert_eq!(request.peer_addr(), Some(SocketAddr::from(([127, 0, 0, 1], 0))));
    /// ```
    pub fn with_local_addr(mut self) -> Self {
        self.extensions_mut()
            .insert(crate::middleware::PeerAddress(std::net::SocketAddr::from(
                ([127, 0, 0, 1], 0),
            )));
        self
    }

    /// Attempts to load the "remote" address for this request.  This is
    /// determined in the following priority:
    ///
    /// 1. The [`Forwarded` header] `for` key, if present;
    /// 2. The _first_ item of the _first_ `X-Forwarded-For` header, if present;
    /// 3. The peer address, if loaded through the hyper service stack (see
    ///    [`Self::peer_addr`]).
    ///
    /// # Note
    /// The client may (maliciously) include either the `Forwarded` header
    /// or the `X-Forwarded-For` header, if your reverse proxy does not filter
    /// either.  Be wary of this when configuring your reverse proxy to provide
    /// the correct address.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// use std::net::IpAddr;
    /// let request = Request::get("/").unwrap()
    ///     .with_header("Forwarded", "for=10.0.0.3").unwrap();
    /// assert_eq!(request.remote(), Some(IpAddr::from([10, 0, 0, 3])));
    /// ```
    ///
    /// ```rust
    /// # use under::*;
    /// use std::net::IpAddr;
    /// let request = Request::get("/").unwrap()
    ///     .with_header("X-Forwarded-For", "10.0.0.3").unwrap();
    /// assert_eq!(request.remote(), Some(IpAddr::from([10, 0, 0, 3])));
    /// ```
    ///
    /// ```rust
    /// # use under::*;
    /// use std::net::IpAddr;
    /// let request = Request::get("/").unwrap()
    ///     .with_local_addr();
    /// assert_eq!(request.remote(), Some(IpAddr::from([127, 0, 0, 1])));
    /// ```
    pub fn remote(&self) -> Option<std::net::IpAddr> {
        use std::net::IpAddr;
        fn forwarded_header(request: &Request) -> Option<IpAddr> {
            request
                .header("Forwarded")
                .and_then(|s| s.to_str().ok())?
                .split(';')
                .find_map(|s| {
                    s.trim()
                        .strip_prefix("for=")
                        .and_then(|s| s.trim_matches('"').parse::<IpAddr>().ok())
                })
        }

        fn x_forwarded_for_header(request: &Request) -> Option<IpAddr> {
            request
                .header("X-Forwarded-For")
                .and_then(|s| s.to_str().ok())?
                .split(',')
                .next()
                .and_then(|s| s.trim().parse::<IpAddr>().ok())
        }

        forwarded_header(self)
            .or_else(|| x_forwarded_for_header(self))
            .or_else(|| self.peer_addr().map(|addr| addr.ip()))
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

    #[doc(hidden)]
    pub fn body_mut(&mut self) -> &mut hyper::Body {
        self.0.body_mut()
    }
}

has_body!(Request);
has_extensions!(Request);
has_headers!(Request);

impl From<http::Request<hyper::Body>> for Request {
    fn from(r: http::Request<hyper::Body>) -> Self {
        Request(r)
    }
}
