#[async_trait]
/// A trait implemented for both [`crate::Request`] and [`crate::Response`]
/// that allows an interaction with their headers, since they both share common
/// abilities.
pub trait HasHeaders: Sized {
    /// Returns a reference to the associated header field map.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use under::*;
    /// let response = Response::default();
    /// assert!(response.headers().is_empty());
    /// ```
    fn headers(&self) -> &http::HeaderMap<http::HeaderValue>;
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
    fn headers_mut(&mut self) -> &mut http::HeaderMap<http::HeaderValue>;

    /// Retrieves the given header specified here.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// let response = Response::text("hello, world");
    /// let content_type = response.header("Content-Type").unwrap();
    /// assert_eq!(content_type.as_bytes(), b"text/plain; charset=utf-8");
    /// ```
    fn header<H: http::header::AsHeaderName>(&self, key: H) -> Option<&http::HeaderValue> {
        self.headers().get(key)
    }

    /// Sets the given header to the given value.  If there already was a
    /// header, it is replaced with the given value.
    ///
    /// # Errors
    /// If the given value cannot be converted into a header value, this will
    /// return an error.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// # use http::header::*;
    /// let mut response = Response::default();
    /// response.set_header(LOCATION, "/").unwrap();
    /// let location: Option<&[u8]> = response.header(LOCATION).map(|v| v.as_bytes());
    /// assert_eq!(location, Some(&b"/"[..]));
    /// ```
    fn set_header<H, V>(&mut self, key: H, value: V) -> Result<(), http::Error>
    where
        H: http::header::IntoHeaderName,
        V: TryInto<http::HeaderValue>,
        http::Error: From<<V as TryInto<http::HeaderValue>>::Error>,
    {
        self.headers_mut().insert(key, value.try_into()?);
        Ok(())
    }

    /// Sets the given header, consuming `self` and returning a new version
    /// with the given header.  This can be useful for builder patterns.
    /// Otherwise, this acts the same as [`HasHeaders::set_header`].
    fn with_header<H, V>(mut self, key: H, value: V) -> Result<Self, http::Error>
    where
        H: http::header::IntoHeaderName,
        V: TryInto<http::HeaderValue>,
        http::Error: From<<V as TryInto<http::HeaderValue>>::Error>,
    {
        self.headers_mut().insert(key, value.try_into()?);
        Ok(self)
    }
}

pub(crate) mod sealed {
    pub trait Sealed {}
}
