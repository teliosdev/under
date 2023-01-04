use super::{Middleware, Next};
use crate::{HttpEntity, Request, Response};
use cookie::{Cookie, CookieJar};
use std::pin::Pin;

/// Middleware for loading and setting cookies.
///
/// On every request, it will load a cookie jar from the request's headers, and
/// set the cookies on the response's headers.  However, the response will need
/// the appropriate cookie jar to be set; otherwise, the cookies will not be
/// sent.
///
/// # Example
/// ```rust
/// # use under::*;
/// # use cookie::Cookie;
/// use under::middleware::{CookieMiddleware, CookieExt};
///
/// # #[tokio::main] async fn main() -> Result<(), anyhow::Error> {
/// async fn handler(req: Request) -> Response {
///     Response::empty_200()
///         .with_cookie(Cookie::new("foo", "bar"))
/// }
/// let mut http = under::http();
/// http
///     .with(CookieMiddleware::new())
///     .at("/foo").get(handler);
/// http.prepare();
/// let response = http.handle(Request::get("/foo")?).await?;
/// assert_eq!(response.header("set-cookie").unwrap().to_str().unwrap(), "foo=bar");
/// # Ok(())
/// # }
/// ```
#[derive(Default, Clone)]
pub struct CookieMiddleware {
    _v: (),
}

/// A trait for types that have cookies.  This allows interfacing with their
/// cookie jars.
pub trait CookieExt: self::sealed::Sealed + Sized {
    #[doc(hidden)]
    fn extensions(&self) -> &http::Extensions;
    #[doc(hidden)]
    fn extensions_mut(&mut self) -> &mut http::Extensions;

    /// Returns the extracted cookie jar.  If no cookie jar had been extracted
    /// (i.e., the [`CookieMiddleware`] was not used), then this will return
    /// `None`.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// use under::middleware::CookieExt;
    /// let request = Request::get("/").unwrap();
    /// assert!(request.cookies().is_none());
    /// let request = request.with_cookies(Default::default());
    /// assert!(request.cookies().is_some());
    /// ```
    fn cookies(&self) -> Option<&CookieJar> {
        self.extensions().get::<CookieJar>()
    }

    /// Returns the mutable cookie jar.  If no cookie jar had been extracted
    /// (i.e. the [`CookieMiddleware`] was not used), then this will return
    /// an empty cookie jar.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// use under::middleware::CookieExt;
    /// let mut request = Request::get("/").unwrap();
    /// assert!(request.cookies_mut().iter().count() == 0);
    /// ```
    fn cookies_mut(&mut self) -> &mut CookieJar {
        if self.extensions().get::<CookieJar>().is_none() {
            self.extensions_mut().insert(CookieJar::new());
        }
        self.extensions_mut().get_mut::<CookieJar>().unwrap()
    }

    /// Returns `self` with the given cookie jar.  This replaces (and drops)
    /// the current cookie jar, if one exists.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// use under::middleware::CookieExt;
    /// let request = Request::get("/").unwrap();
    /// assert!(request.cookies().is_none());
    /// let request = request.with_cookies(Default::default());
    /// assert!(request.cookies().is_some());
    /// ```
    #[must_use]
    fn with_cookies(mut self, jar: CookieJar) -> Self {
        self.extensions_mut().insert(jar);
        self
    }

    /// Returns the value for the cookie with the given name.  If no cookie
    /// jar is set, it returns `None`; if no cookie with the given name exists,
    /// it returns `None`; otherwise, it returns its value.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// # use cookie::Cookie;
    /// use under::middleware::CookieExt;
    /// let mut request = Request::get("/").unwrap();
    /// assert!(request.cookie("foo").is_none());
    /// request.cookies_mut().add(Cookie::new("foo", "bar"));
    /// assert_eq!(request.cookie("foo"), Some("bar"));
    /// ```
    fn cookie(&self, name: &str) -> Option<&str> {
        self.cookies()
            .and_then(|c| c.get(name))
            .map(cookie::Cookie::value)
    }

    /// Adds the given cookie to the current cookie jar.  This addition does
    /// add to the delta, and creates the cookie jar if it does not exist.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// # use cookie::Cookie;
    /// use under::middleware::CookieExt;
    /// let mut request = Request::get("/").unwrap();
    /// assert!(request.cookie("foo").is_none());
    /// request.add_cookie(Cookie::new("foo", "bar"));
    /// assert_eq!(request.cookie("foo"), Some("bar"));
    /// ```
    fn add_cookie(&mut self, cookie: Cookie<'static>) {
        self.cookies_mut().add(cookie);
    }

    /// Adds the given cookie to the cookie jar.  This is essentially the same
    /// as [`Self::add_cookie`].
    #[must_use]
    fn with_cookie(mut self, cookie: Cookie<'static>) -> Self {
        self.add_cookie(cookie);
        self
    }
}

impl self::sealed::Sealed for Request {}
impl self::sealed::Sealed for Response {}

impl CookieExt for Request {
    fn extensions(&self) -> &http::Extensions {
        self.extensions()
    }
    fn extensions_mut(&mut self) -> &mut http::Extensions {
        self.extensions_mut()
    }
}

impl CookieExt for Response {
    fn extensions(&self) -> &http::Extensions {
        self.extensions()
    }
    fn extensions_mut(&mut self) -> &mut http::Extensions {
        self.extensions_mut()
    }
}

mod sealed {
    pub trait Sealed {}
}

impl std::fmt::Debug for CookieMiddleware {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CookieMiddleware").finish()
    }
}

impl CookieMiddleware {
    /// Creates a new cookie middleware.  This is provided as an alternative
    /// to `Default`.
    #[must_use]
    pub fn new() -> Self {
        Self { _v: () }
    }
}

#[async_trait]
impl Middleware for CookieMiddleware {
    async fn apply(
        self: Pin<&Self>,
        mut request: Request,
        next: Next<'_>,
    ) -> Result<Response, anyhow::Error> {
        let jar = request
            .headers()
            .get_all("Cookie")
            .into_iter()
            .filter_map(|h| h.to_str().ok())
            .filter_map(|h| Cookie::parse_encoded(h).ok())
            .map(cookie::Cookie::into_owned)
            .fold(CookieJar::new(), |mut jar, cookie| {
                jar.add_original(cookie);
                jar
            });
        request.extensions_mut().insert(jar);
        let mut response = next.apply(request).await?;

        let result_jar = response.extensions_mut().remove::<CookieJar>();

        if let Some(jar) = result_jar {
            let headers = response.headers_mut();
            for cookie in jar.delta() {
                if let Ok(cookie) = cookie.encoded().to_string().try_into() {
                    headers.append("Set-Cookie", cookie);
                }
            }
        }

        Ok(response)
    }
}
