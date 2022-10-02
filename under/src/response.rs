use std::convert::TryFrom;

#[derive(Debug)]
#[must_use]
/// An HTTP response.
///
/// An HTTP Response consists of a head (a status code and some headers), and
/// a body (which may be empty).  This type offers convenient helpers for
/// constructing HTTP responses for you for common use-cases.
///
/// # Examples
///
/// ```rust
/// use under::{Request, Response, HttpEntity};
///
/// // Here, we're defining an endpoint for our server.
/// async fn handle_get(request: Request) -> Result<Response, anyhow::Error> {
///     let target = request.fragment_str("target").unwrap_or("world");
///     Ok(Response::text(format!("hello, {}", target)))
/// }
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), anyhow::Error> {
/// let mut http = under::http();
/// http
///     .at("/hello").get(handle_get)
///     .at("/hello/{target}").get(handle_get);
/// http.prepare();
/// let mut response = http.handle(Request::get("/hello")?).await?;
/// assert_eq!(response.status(), http::StatusCode::OK);
/// let body = response.data(512).into_text().await?;
/// assert_eq!(body, "hello, world");
/// # Ok(())
/// # }
/// ```
pub struct Response(http::Response<hyper::Body>);

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

impl Response {
    /// Creates an empty response with a status code of 200.
    ///
    /// See [`Response::empty_status`] for more information.
    ///
    /// # Example
    /// ```rust
    /// # use under::*;
    /// let response = Response::empty_200();
    /// assert_eq!(response.status(), http::StatusCode::OK);
    /// ```
    pub fn empty_200() -> Self {
        Self::empty_status(http::StatusCode::OK)
    }

    /// Creates an empty response with a status code of 204.
    ///
    /// See [`Response::empty_status`] for more information.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// let response = Response::empty_204();
    /// assert_eq!(response.status(), http::StatusCode::NO_CONTENT);
    /// ```
    pub fn empty_204() -> Self {
        Response::empty_status(http::StatusCode::NO_CONTENT)
    }

    /// Creates an empty response with a status code of 404.
    ///
    /// See [`Response::empty_status`] for more information.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// let response = Response::empty_404();
    /// assert_eq!(response.status(), http::StatusCode::NOT_FOUND);
    /// ```
    pub fn empty_404() -> Self {
        Response::empty_status(http::StatusCode::NOT_FOUND)
    }

    /// Creates an empty response with a status code of 500.
    ///
    /// See [`Response::empty_status`] for more information.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// let response = Response::empty_500();
    /// assert_eq!(response.status(), http::StatusCode::INTERNAL_SERVER_ERROR);
    /// ```
    pub fn empty_500() -> Self {
        Response::empty_status(http::StatusCode::INTERNAL_SERVER_ERROR)
    }

    /// Creates a redirect (using See Other) to the given location.
    ///
    /// # Errors
    /// This attempts to convert the location into a [`http::HeaderValue`];
    /// however, the conversion may fail (for reasons specified on
    /// [`http::HeaderValue::from_str`]).  It may also fail to construct the
    /// underlying response.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// let response = Response::see_other("/foo").unwrap();
    /// assert_eq!(response.status(), http::StatusCode::SEE_OTHER);
    /// ```
    pub fn see_other<T>(location: T) -> Result<Self, http::Error>
    where
        http::HeaderValue: TryFrom<T>,
        <http::HeaderValue as TryFrom<T>>::Error: Into<http::Error>,
    {
        Ok(Response(
            http::Response::builder()
                .status(http::StatusCode::SEE_OTHER)
                .header(http::header::LOCATION, location)
                .body(hyper::Body::empty())?,
        ))
    }

    /// Creates a permanent redirect to the given location.
    ///
    /// # Errors
    /// This attempts to convert the location into a
    /// [`http::HeaderValue`]; however, the conversion may fail (for
    /// reasons specified on [`http::HeaderValue::from_str`]).  It may also
    /// fail to construct the underlying response.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// let response = Response::permanent_redirect("/foo").unwrap();
    /// assert_eq!(response.status(), http::StatusCode::PERMANENT_REDIRECT);
    /// ```
    pub fn permanent_redirect<T>(location: T) -> Result<Self, http::Error>
    where
        http::HeaderValue: TryFrom<T>,
        <http::HeaderValue as TryFrom<T>>::Error: Into<http::Error>,
    {
        Ok(Response(
            http::Response::builder()
                .status(http::StatusCode::PERMANENT_REDIRECT)
                .header(http::header::LOCATION, location)
                .body(hyper::Body::empty())?,
        ))
    }

    /// Creates a temporary redirect to the given location.
    ///
    /// # Errors
    /// This attempts to convert the location into a
    /// [`http::HeaderValue`]; however, the conversion may fail (for
    /// reasons specified on [`http::HeaderValue::from_str`]).  It may also
    /// fail to construct the underlying response.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// let response = Response::temporary_redirect("/foo").unwrap();
    /// assert_eq!(response.status(), http::StatusCode::TEMPORARY_REDIRECT);
    /// ```
    pub fn temporary_redirect<T>(location: T) -> Result<Self, http::Error>
    where
        http::HeaderValue: TryFrom<T>,
        <http::HeaderValue as TryFrom<T>>::Error: Into<http::Error>,
    {
        Ok(Response(
            http::Response::builder()
                .status(http::StatusCode::TEMPORARY_REDIRECT)
                .header(http::header::LOCATION, location)
                .body(hyper::Body::empty())?,
        ))
    }

    /// Creates a response with an empty body and a set status.  The
    /// Content-Type is not set.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// let response = Response::empty_status(http::StatusCode::NOT_FOUND);
    /// assert_eq!(response.status(), http::StatusCode::NOT_FOUND);
    /// ```
    pub fn empty_status(status: http::StatusCode) -> Self {
        Response(
            http::Response::builder()
                .status(status)
                .body(hyper::Body::empty())
                .unwrap(),
        )
    }

    /// Creates a response with the given text body.  The returned response
    /// has a `Content-Type` of `text/plain; charset=utf-8`.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// let response = Response::text("hello, world");
    /// ```
    pub fn text<V: Into<String>>(body: V) -> Self {
        Response(
            http::Response::builder()
                .header(http::header::CONTENT_TYPE, "text/plain; charset=utf-8")
                .body(body.into().into())
                .unwrap(),
        )
    }

    /// Creates a response with the given JSON body.  The returned response
    /// has a `Content-Type` of `application/json; charset=utf-8`.
    ///
    /// # Errors
    /// This errors if the underlying JSON serialization fails; and it will
    /// return that exact error.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// let response = Response::json(&serde_json::json!({ "hello": "world" }))?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    #[cfg(feature = "json")]
    pub fn json<V: serde::Serialize>(body: &V) -> Result<Self, serde_json::Error> {
        let value = serde_json::to_string(body)?;
        Ok(Response(
            http::Response::builder()
                .header(
                    http::header::CONTENT_TYPE,
                    "application/json; charset=utf-8",
                )
                .body(value.into())
                .unwrap(),
        ))
    }

    /// Sets the current responses's status code.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// let mut response = Response::empty_404();
    /// response.set_status(http::StatusCode::OK);
    /// assert_eq!(response.status(), http::StatusCode::OK);
    /// ```
    pub fn set_status<S: Into<http::StatusCode>>(&mut self, status: S) {
        *self.0.status_mut() = status.into();
    }

    /// Returns a response with the new status code.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// let response = Response::empty_404();
    /// let response = response.with_status(http::StatusCode::OK);
    /// assert_eq!(response.status(), http::StatusCode::OK);
    /// ```
    pub fn with_status<S: Into<http::StatusCode>>(mut self, status: S) -> Self {
        *self.0.status_mut() = status.into();
        Response(self.0)
    }

    /// Returns state information provided by the
    /// [`crate::middleware::StateMiddleware`] middleware.  This is a
    /// shortcut to retrieving the [`crate::middleware::State`]
    /// extension from the response.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// use under::middleware::State;
    /// let mut response = Response::empty_200();
    /// response.extensions_mut().insert(State(123u32));
    /// assert_eq!(response.state::<u32>(), Some(&123u32));
    /// ```
    pub fn state<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.ext::<crate::middleware::State<T>>().map(|v| &v.0)
    }

    /// Retrieves a specific extension from the extensions map.  This is
    /// the same as calling [`Self::extensions`].`get` wit the given
    /// type parameter.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// let mut response = Response::empty_200();
    /// assert_eq!(response.ext::<u32>(), None);
    /// ```
    pub fn ext<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.extensions().get::<T>()
    }

    /// Retrieves a mutable reference to the specific extension from the
    /// extensions map.  This is the same as calling
    /// [`Self::extensions_mut`].`get_mut` with the given type
    /// parameter.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// let mut response = Response::empty_200();
    /// assert_eq!(response.ext_mut::<u32>(), None);
    /// ```
    pub fn ext_mut<T: Send + Sync + 'static>(&mut self) -> Option<&mut T> {
        self.extensions_mut().get_mut::<T>()
    }

    /// Sets the value of the specific extension in the extensions map.
    /// This is the same as calling [`Self::extensions_mut`].`insert`
    /// with the given parameter.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// let mut response = Response::empty_200();
    /// response.set_ext(123u32);
    /// assert_eq!(response.ext::<u32>(), Some(&123u32));
    /// ```
    pub fn set_ext<T: Send + Sync + 'static>(&mut self, value: T) -> &mut Self {
        self.extensions_mut().insert(value);
        self
    }

    /// Sets the value of the specific extension in the extensions map,
    /// consuming `self`, and then returning the new value.  This is
    /// the same as calling [`Self::set_ext`], but it consumes `self`.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// let response = Response::empty_200();
    /// let response = response.with_ext(123u32);
    /// assert_eq!(response.ext::<u32>(), Some(&123u32));
    /// ```
    pub fn with_ext<T: Send + Sync + 'static>(mut self, value: T) -> Self {
        self.set_ext(value);
        self
    }

    /// Removes the specific extension from the extensions map.  This is
    /// the same as calling [`Self::extensions_mut`].`remove` with the
    /// given type parameter.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// let mut response = Response::empty_200()
    ///     .with_ext(123u32);
    /// assert_eq!(response.ext::<u32>(), Some(&123u32));
    /// response.remove_ext::<u32>();
    /// assert_eq!(response.ext::<u32>(), None);
    /// ```
    pub fn remove_ext<T: Send + Sync + 'static>(&mut self) -> Option<T> {
        self.extensions_mut().remove::<T>()
    }

    /// Removes the specific extension from the extensions map,
    /// consuming `self`, and then returning the removed value.  This
    /// is the same as calling [`Self::remove_ext`], but it consumes
    /// `self`.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// let response = Response::empty_200()
    ///     .with_ext(123u32);
    /// assert_eq!(response.ext::<u32>(), Some(&123u32));
    /// let response = response.without_ext::<u32>();
    /// assert_eq!(response.ext::<u32>(), None);
    /// ```
    pub fn without_ext<T: Send + Sync + 'static>(mut self) -> Self {
        self.remove_ext::<T>();
        self
    }

    forward! {
        /// Returns the [`http::StatusCode`].
        ///
        /// # Examples
        ///
        /// ```rust
        /// # use under::*;
        /// let response = Response::default();
        /// assert_eq!(response.status(), http::StatusCode::OK);
        /// ```
        pub fn status(&self) -> http::StatusCode;
        /// Returns a reference to the associated extensions.
        ///
        /// # Examples
        ///
        /// ```rust
        /// # use under::*;
        /// let response = Response::default();
        /// assert!(response.extensions().get::<i32>().is_none());
        /// ```
        pub fn extensions(&self) -> &http::Extensions;
        /// Returns a mutable reference to the associated extensions.
        ///
        /// # Examples
        ///
        /// ```rust
        /// # use under::*;
        /// let mut response = Response::default();
        /// response.extensions_mut().insert("hello");
        /// assert_eq!(response.extensions().get(), Some(&"hello"));
        /// ```
        pub fn extensions_mut(&mut self) -> &mut http::Extensions;
        /// Returns a reference to the associated header field map.
        ///
        /// # Examples
        ///
        /// ```rust
        /// # use under::*;
        /// let response = Response::default();
        /// assert!(response.headers().is_empty());
        /// ```
        pub fn headers(&self) -> &http::HeaderMap<http::HeaderValue>;
        /// Returns a mutable reference to the associated header field map.
        ///
        /// # Examples
        ///
        /// ```
        /// # use under::*;
        /// # use http::header::*;
        /// let mut response = Response::default();
        /// response.headers_mut().insert(HOST, HeaderValue::from_static("world"));
        /// assert!(!response.headers().is_empty());
        /// ```
        pub fn headers_mut(&mut self) -> &mut http::HeaderMap<http::HeaderValue>;
    }
}

impl crate::HttpEntity for Response {
    #[inline]
    fn body_mut(&mut self) -> &mut hyper::Body {
        self.0.body_mut()
    }
    #[inline]
    fn headers(&self) -> &http::HeaderMap<http::HeaderValue> {
        self.0.headers()
    }
    #[inline]
    fn headers_mut(&mut self) -> &mut http::HeaderMap<http::HeaderValue> {
        self.0.headers_mut()
    }
}

impl Default for Response {
    fn default() -> Self {
        Response(
            http::Response::builder()
                .body(hyper::Body::empty())
                .unwrap(),
        )
    }
}

impl From<http::Response<hyper::Body>> for Response {
    fn from(hy: http::Response<hyper::Body>) -> Self {
        Response(hy)
    }
}

impl From<Response> for http::Response<hyper::Body> {
    fn from(this: Response) -> Self {
        this.0
    }
}

/// Converts the current type into a [`crate::Response`].
///
/// This assumes that the conversion into a response is fallible
/// (as it often is).  This is used instead of `TryFrom` because
/// `TryFrom<Result<T, E>>` is not implemented for `Result<T, E>`.
///
/// This uses `anyhow::Error` as the error type for a few reasons:
///
/// 1. [`anyhow::Error`] does not implement [`std::error::Error`].
/// 2. [`std::convert::Infallible`]/[`!`] does not implement
///    `Into<T>`/`From<T>`.
/// 3. I can't figure out how to reconcile the two such that
///    `IntoResponse` can be implemented for `Result<R, E>` where
///    `R: IntoResponse`; especially if `R` is `Response`, as the
///    `IntoResponse` implementation for that would have the error
///    type be `Infallible`.
pub trait IntoResponse {
    /// Converts the current type into a response.
    fn into_response(self) -> Result<Response, anyhow::Error>;
}

impl IntoResponse for Response {
    fn into_response(self) -> Result<Response, anyhow::Error> {
        Ok(self)
    }
}

impl<R, E> IntoResponse for Result<R, E>
where
    R: IntoResponse,
    E: Into<anyhow::Error>,
{
    fn into_response(self) -> Result<Response, anyhow::Error> {
        self.map_err(Into::into)
            .and_then(|r| r.into_response().map_err(Into::into))
    }
}

impl IntoResponse for std::convert::Infallible {
    fn into_response(self) -> Result<Response, anyhow::Error> {
        match self {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_response() {
        let response = Response::empty_500();

        assert!(Ok::<_, std::convert::Infallible>(response)
            .into_response()
            .is_ok());
    }
}
