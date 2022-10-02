use crate::data::DataStream;
use crate::UnderError;

/// A HTTP Entity.
///
/// This is either a request or a response.  This represents common, shared
/// functionality between the two, such as accessing headers and the body.
#[async_trait::async_trait]
pub trait HttpEntity: Sized {
    /// Returns a mutable reference to the body of the request.  This is used
    /// for all other methods in `HasBody`.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// # #[tokio::main] async fn main() -> Result<(), anyhow::Error> {
    /// let mut response = Response::text("hello");
    /// let body = std::mem::replace(response.body_mut(), hyper::Body::empty());
    /// let body = hyper::body::to_bytes(body).await?;
    /// assert_eq!(&body[..], b"hello");
    /// # Ok(())
    /// # }
    fn body_mut(&mut self) -> &mut hyper::Body;

    /// Sets the body of the request to the given body.  This causes the
    /// previous body to be dropped in place.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// # #[tokio::main] async fn main() -> Result<(), anyhow::Error> {
    /// let mut response = Response::default();
    /// response.set_body("foo");
    /// let body = hyper::body::to_bytes(response.take_body()).await?;
    /// assert_eq!(&body[..], b"foo");
    /// # Ok(())
    /// # }
    /// ```
    ///
    fn set_body<I: Into<hyper::Body>>(&mut self, body: I) -> &mut Self {
        *self.body_mut() = body.into();
        self
    }

    /// Sets the body of the request to the given body, consuming
    /// `self`.  This causes the previous body to be dropped in place.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// # #[tokio::main] async fn main() -> Result<(), anyhow::Error> {
    /// let mut response = Response::default()
    ///     .with_body("foo");
    /// let body = hyper::body::to_bytes(response.take_body()).await?;
    /// assert_eq!(&body[..], b"foo");
    /// # Ok(())
    /// # }
    /// ```
    fn with_body<I: Into<hyper::Body>>(mut self, body: I) -> Self {
        *self.body_mut() = body.into();
        self
    }

    /// Takes the body from this request, and replaces it with an empty body.
    /// The previous body is replaced with an empty body; thus, attempting to
    /// read the body more than once will cause successive attempts to fail.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// # #[tokio::main] async fn main() -> Result<(), anyhow::Error> {
    /// let mut response = Response::default().with_body("foo");
    /// let body = hyper::body::to_bytes(response.take_body()).await?;
    /// assert_eq!(&body[..], b"foo");
    /// let body = hyper::body::to_bytes(response.take_body()).await?;
    /// assert_eq!(&body[..], b"");
    /// # Ok(())
    /// # }
    /// ```
    fn take_body(&mut self) -> hyper::Body {
        std::mem::replace(self.body_mut(), hyper::Body::empty())
    }

    /// Replaces the contents of the body with the given JSON body.  Note
    /// that this does _not_ update the Content-Type; the caller is responsible
    /// for that.
    ///
    /// # Errors
    /// This errors if the underlying JSON serialization fails; and it will
    /// return that exact error.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// # fn main() -> Result<(), anyhow::Error> {
    /// let mut response = Response::empty_404();
    /// response.set_json(&serde_json::json!({ "error": 404 }))?;
    /// assert_eq!(response.header(http::header::CONTENT_TYPE), None);
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "json")]
    #[doc(cfg(feature = "json"))]
    fn set_json<V: serde::Serialize>(
        &mut self,
        new_body: &V,
    ) -> Result<&mut Self, serde_json::Error> {
        let value = serde_json::to_string(new_body)?;
        Ok(self.set_body(value))
    }

    /// Replaces the contents of the body with the given JSON body,
    /// consuming `self`.  Note that this does _not_ update the
    /// Content-Type; the caller is responsible for that.
    ///
    /// # Errors
    /// This errors if the underlying JSON serialization fails; and it will
    /// return that exact error.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// # fn main() -> Result<(), anyhow::Error> {
    /// let mut response = Response::empty_404();
    /// let response = response.with_json(&serde_json::json!({ "error": 404 }))?;
    /// assert_eq!(response.header(http::header::CONTENT_TYPE), None);
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "json")]
    #[doc(cfg(feature = "json"))]
    fn with_json<V: serde::Serialize>(self, new_body: &V) -> Result<Self, serde_json::Error> {
        let value = serde_json::to_string(new_body)?;
        Ok(self.with_body(value))
    }

    /// Replaces the contents of the body with the given CBOR body.  Note
    /// that this does _not_ update the Content-Type; the caller is responsible
    /// for that.
    ///
    /// # Errors
    /// This errors if the underlying CBOR serialization fails; and it will
    /// return that exact error.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// # use ciborium::cbor;
    /// # fn main() -> Result<(), anyhow::Error> {
    /// let mut response = Response::empty_404();
    /// response.set_cbor(&cbor!({ "error" => 404 }).unwrap())?;
    /// assert_eq!(response.header(http::header::CONTENT_TYPE), None);
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "cbor")]
    #[doc(cfg(feature = "cbor"))]
    fn set_cbor<V: serde::Serialize>(
        &mut self,
        new_body: &V,
    ) -> Result<&mut Self, ciborium::ser::Error<std::io::Error>> {
        let mut out = vec![];
        ciborium::ser::into_writer(new_body, &mut out)?;
        Ok(self.set_body(out))
    }

    /// Replaces the contents of the body with the given CBOR body, consuming
    /// `self`.  Note that this does _not_ update the Content-Type; the caller
    /// is responsible for that.
    ///
    /// # Errors
    /// This errors if the underlying CBOR serialization fails; and it will
    /// return that exact error.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// # use ciborium::cbor;
    /// # fn main() -> Result<(), anyhow::Error> {
    /// let mut response = Response::empty_404();
    /// let response = response.with_cbor(&cbor!({ "error" => 404 }).unwrap())?;
    /// assert_eq!(response.header(http::header::CONTENT_TYPE), None);
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "cbor")]
    #[doc(cfg(feature = "cbor"))]
    fn with_cbor<V: serde::Serialize>(self, new_body: &V) -> Result<Self, anyhow::Error> {
        let mut out = vec![];
        ciborium::ser::into_writer(new_body, &mut out)?;
        Ok(self.with_body(out))
    }

    /// Replaces the contents of the body with the given MessagePack body.  Note
    /// that this does _not_ update the Content-Type; the caller is responsible
    /// for that.
    ///
    /// # Errors
    /// This errors if the underlying MessagePack serialization fails; and it
    /// will return that exact error.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// #[derive(serde::Serialize)]
    /// struct Err { error: u16 }
    /// # fn main() -> Result<(), anyhow::Error> {
    /// let mut response = Response::empty_404();
    /// response.set_msgpack(&Err { error: 404 })?;
    /// assert_eq!(response.header(http::header::CONTENT_TYPE), None);
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "msgpack")]
    #[doc(cfg(feature = "msgpack"))]
    fn set_msgpack<V: serde::Serialize>(
        &mut self,
        new_body: &V,
    ) -> Result<&mut Self, rmp_serde::encode::Error> {
        let mut out = vec![];
        rmp_serde::encode::write_named(&mut out, new_body)?;
        Ok(self.set_body(out))
    }

    /// Replaces the contents of the body with the given MessagePack body,
    /// consuming `self`.  Note that this does _not_ update the Content-Type;
    /// the caller is responsible for that.
    ///
    /// # Errors
    /// This errors if the underlying MessagePack serialization fails; and it
    /// will return that exact error.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// #[derive(serde::Serialize)]
    /// struct Err { error: u16 }
    /// # fn main() -> Result<(), anyhow::Error> {
    /// let mut response = Response::empty_404();
    /// let response = response.with_msgpack(&Err { error: 404 })?;
    /// assert_eq!(response.header(http::header::CONTENT_TYPE), None);
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "msgpack")]
    #[doc(cfg(feature = "msgpack"))]
    fn with_msgpack<V: serde::Serialize>(
        self,
        new_body: &V,
    ) -> Result<Self, rmp_serde::encode::Error> {
        let mut out = vec![];
        rmp_serde::encode::write_named(&mut out, new_body)?;
        Ok(self.with_body(out))
    }

    /// Creates a data stream of the body.  This consumes the body, and
    /// produces a stream that can then be read from.  A limit must be
    /// provided, which is the maximum number of bytes that can be read
    /// from the stream.  For most operations, exceeding this limit
    /// will cause an error.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// # #[tokio::main] async fn main() -> Result<(), anyhow::Error> {
    /// let mut response = Response::text("hello, world");
    /// let data = response.data(1_000_000)
    ///     .into_text().await?;
    /// assert_eq!(&data[..], "hello, world");
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ```rust
    /// # use under::*;
    /// # #[tokio::main] async fn main() -> Result<(), anyhow::Error> {
    /// let mut response = Response::text("hello, world");
    /// let data = response.data(1)
    ///    .into_text().await;
    /// assert!(data.is_err());
    /// # Ok(())
    /// # }
    /// ```
    fn data(&mut self, limit: u64) -> DataStream {
        DataStream::new(self.take_body(), limit)
    }

    /// Returns a reference to the associated header field map.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use under::*;
    /// let request = Request::get("/").unwrap();
    /// assert!(request.headers().is_empty());
    /// ```
    fn headers(&self) -> &http::HeaderMap<http::HeaderValue>;
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

    /// Retrieves all potential values for the given header specified
    /// here.
    fn header_all<H: http::header::AsHeaderName>(
        &self,
        key: H,
    ) -> http::header::GetAll<'_, http::HeaderValue> {
        self.headers().get_all(key)
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
    /// assert_eq!(location, Some(b"/".as_ref()));
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
    /// Otherwise, this acts the same as [`Self::set_header`].
    fn with_header<H, V>(mut self, key: H, value: V) -> Result<Self, http::Error>
    where
        H: http::header::IntoHeaderName,
        V: TryInto<http::HeaderValue>,
        http::Error: From<<V as TryInto<http::HeaderValue>>::Error>,
    {
        self.headers_mut().insert(key, value.try_into()?);
        Ok(self)
    }

    /// Sets the given header to the given value.  If there already was a
    /// header, it is appended with the given value.
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
    /// response.add_header(LOCATION, "/hello").unwrap();
    /// let location: Vec<&[u8]> = response.header_all(LOCATION)
    ///     .into_iter()
    ///     .map(|v| v.as_bytes())
    ///     .collect::<Vec<_>>();
    /// assert_eq!(location, vec![b"/".as_ref(), b"/hello".as_ref()]);
    /// ```
    fn add_header<H, V>(&mut self, key: H, value: V) -> Result<(), http::Error>
    where
        H: http::header::IntoHeaderName,
        V: TryInto<http::HeaderValue>,
        http::Error: From<<V as TryInto<http::HeaderValue>>::Error>,
    {
        self.headers_mut().append(key, value.try_into()?);
        Ok(())
    }

    /// Sets the given header, consuming `self` and returning a new version
    /// with the given header.  This can be useful for builder patterns.
    /// Otherwise, this acts the same as [`Self::add_header`].
    fn with_add_header<H, V>(mut self, key: H, value: V) -> Result<Self, http::Error>
    where
        H: http::header::IntoHeaderName,
        V: TryInto<http::HeaderValue>,
        http::Error: From<<V as TryInto<http::HeaderValue>>::Error>,
    {
        self.headers_mut().append(key, value.try_into()?);
        Ok(self)
    }

    /// Retrieves the content type of the body.  This is normally pulled from
    /// the `Content-Type` header of the request, and parsed into
    /// a mime; if the header does not exist, or is not a proper mime type,
    /// this will return `None`.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// # #[tokio::main] async fn main() -> Result<(), anyhow::Error> {
    /// let mut request = Request::get("/").unwrap();
    /// assert!(request.content_type().is_none());
    /// let request = request.with_header(http::header::CONTENT_TYPE, "application/json")?;
    /// let ctype = request.content_type();
    /// assert_eq!(ctype.as_ref().map(|m| m.essence_str()), Some("application/json"));
    /// # Ok(())
    /// # }
    /// ```
    fn content_type(&self) -> Option<mime::Mime> {
        let content_type = self.headers().get(http::header::CONTENT_TYPE)?;
        let content_type = content_type.to_str().ok()?;
        let content_type = content_type.parse::<mime::Mime>().ok()?;
        Some(content_type)
    }

    /// Attempts to parse the body based off of the content-type header;
    /// currently, it can sniff either `application/json` or
    /// `application/x-www-form-urlencoded`.  If the content-type is either of
    /// those, it forwards the call to the respective functions
    /// ([`Self::as_json`] and [`Self::as_form`]).  If it cannot find
    /// the content type, or the content type is not one of those two, it will
    /// return an error.
    ///
    /// # Note
    /// This provides an implicit limit of 3,000,000 bytes. If the body
    /// exceeds this limit, then this function will return an error.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// #[derive(Debug, serde::Deserialize, FromForm, PartialEq, Eq)]
    /// struct Form {
    ///    hello: String,
    /// }
    ///
    /// # #[tokio::main] async fn main() -> Result<(), anyhow::Error> {
    /// let mut response = Response::text(r#"{"hello": "world"}"#);
    /// response
    ///     .set_header(http::header::CONTENT_TYPE, "application/json")?;
    /// let body = response.as_sniff::<Form>(512).await?;
    /// let expected = Form { hello: "world".to_string() };
    /// assert_eq!(body, expected);
    /// # Ok(())
    /// # }
    /// ```

    /// Attempts to parse the body based off of the content-type header;
    /// currently, it can sniff any activated serde features (e.g. `json`,
    /// `cbor`, `msgpack`).  If the content-type is one of those, it forwards
    /// the call to the respective functions ([`DataStream::into_json`],
    /// [`DataStream::into_cbor`], [`DataStream::into_msgpack`]), thereby
    /// consuming the body.  If it cannot find the content type, or the content
    /// type is not one of those, it will return an error.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// #[derive(Debug, serde::Deserialize, PartialEq, Eq)]
    /// struct Form {
    ///   hello: String,
    /// }
    ///
    /// # #[tokio::main] async fn main() -> Result<(), anyhow::Error> {
    /// let mut response = Response::text(r#"{"hello": "world"}"#);
    /// response
    ///  .set_header(http::header::CONTENT_TYPE, "application/json")?;
    /// let body = response.as_sniff::<Form>(512).await?;
    /// let expected = Form { hello: "world".to_string() };
    /// assert_eq!(body, expected);
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(all(feature = "serde"))]
    #[doc(cfg(all(feature = "serde")))]
    async fn as_sniff<T: serde::de::DeserializeOwned>(
        &mut self,
        limit: u64,
    ) -> Result<T, UnderError> {
        sniff_serde(self, limit).await
    }

    /// Attempts to parse the body based off of the content type header;
    /// currently, it can sniff any activated serde features (e.g. `json`,
    /// `cbor`, `msgpack`), or x-www-form-urlencoded.  If the content-type is
    /// one of those, it forwards the call to the respective functions
    /// ([`DataStream::into_json`], [`DataStream::into_cbor`],
    /// [`DataStream::into_msgpack`], [`DataStream::into_form`]), thereby
    /// consuming the body.  If it cannot find the content type, or the content
    /// type is not one of those, it will return an error.
    ///
    /// This functions similarly to [`HttpEntity::as_sniff`], but it also can
    /// parse `x-www-form-urlencoded` content types as well.
    #[cfg(all(feature = "serde", feature = "from_form"))]
    #[doc(cfg(all(feature = "serde", feature = "from_form")))]
    async fn as_sniff_form<T: serde::de::DeserializeOwned + crate::FromForm>(
        &mut self,
        limit: u64,
    ) -> Result<T, UnderError> {
        let ctype = self.content_type();
        if ctype.as_ref().map(|m| m.essence_str()) == Some("application/x-www-form-urlencoded") {
            self.data(limit).into_form().await
        } else {
            sniff_serde(self, limit).await
        }
    }
}

#[cfg(feature = "serde")]
#[doc(cfg(feature = "serde"))]
async fn sniff_serde<E: HttpEntity, T: serde::de::DeserializeOwned>(
    entity: &mut E,
    limit: u64,
) -> Result<T, UnderError> {
    let ctype = entity.content_type();
    let essence = ctype.as_ref().map(|m| m.essence_str());

    match essence {
        #[cfg(feature = "json")]
        Some("application/json") => entity.data(limit).into_json().await,
        #[cfg(feature = "cbor")]
        Some("application/cbor") => entity.data(limit).into_cbor().await,
        #[cfg(feature = "msgpack")]
        Some("application/msgpack") => entity.data(limit).into_msgpack().await,
        _ => Err(UnderError::UnsupportedMediaType(ctype)),
    }
}
