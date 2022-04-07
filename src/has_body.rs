macro_rules! has_body {
    ($ty:ty) => {
        impl $ty {
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
            pub fn set_body<I: Into<hyper::Body>>(&mut self, body: I) -> &mut Self {
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
            pub fn with_body<I: Into<hyper::Body>>(mut self, body: I) -> Self {
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
            pub fn take_body(&mut self) -> hyper::Body {
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
            pub fn set_json<V: serde::Serialize>(
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
            pub fn with_json<V: serde::Serialize>(
                self,
                new_body: &V,
            ) -> Result<Self, serde_json::Error> {
                let value = serde_json::to_string(new_body)?;
                Ok(self.with_body(value))
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
            /// ```
            pub fn data(&mut self, limit: u64) -> DataStream {
                DataStream::new(self.take_body(), limit)
            }

            /// Converts the contents of the body into a byte buffer, which can
            /// then be consumed downstream.
            ///
            /// # Note
            /// This provides an implicit limit of 3,000,000 bytes. If the body
            /// exceeds this limit, then this function will return an error.
            ///
            /// # Examples
            /// ```rust
            /// # use under::*;
            /// # #[tokio::main] async fn main() -> Result<(), anyhow::Error> {
            /// let mut response = Response::text("hello, world");
            /// let body = response.as_bytes().await?;
            /// assert_eq!(&body[..], b"hello, world");
            /// # Ok(())
            /// # }
            /// ```
            pub async fn as_bytes(&mut self) -> Result<Vec<u8>, crate::UnderError> {
                self.data(3_000_000)
                    .into_bytes()
                    .await
                    .map_err(crate::UnderError::ReadBody)
            }

            /// Converts the contents of the body into a UTF-8 string.  This
            /// assumes that the request body is already UTF-8, or a UTF-8 compatible
            /// encoding, and does not check the content-type to make sure.  If that
            /// is a concern, use [`Self::as_bytes`], and handle the conversion
            /// yourself; or, if it's a common occurrance, open a ticket, with your
            /// use-case and a proposed solution.
            ///
            /// # Note
            /// This provides an implicit limit of 3,000,000 bytes. If the body
            /// exceeds this limit, then this function will return an error.
            ///
            /// # Examples
            /// ```rust
            /// # use under::*;
            /// # #[tokio::main] async fn main() -> Result<(), anyhow::Error> {
            /// let mut response = Response::text("hello, world");
            /// let body = response.as_text().await?;
            /// assert_eq!(body, "hello, world");
            /// # Ok(())
            /// # }
            /// ```
            pub async fn as_text(&mut self) -> Result<String, crate::UnderError> {
                self.data(3_000_000)
                    .into_text()
                    .await
                    .map_err(crate::UnderError::ReadBody)
            }

            /// Parses the contents of the body as JSON, deserializing it into the
            /// given value.  JSON has strict limits on the bytes/characters allowed
            /// for serialization/deserialization, so the charset should not matter.
            ///
            /// # Note
            /// This provides an implicit limit of 3,000,000 bytes. If the body
            /// exceeds this limit, then this function will return an error.
            ///
            /// # Examples
            /// ```rust
            /// # use under::*;
            /// # #[tokio::main] async fn main() -> Result<(), anyhow::Error> {
            /// let mut response = Response::text(r#"{"hello": "world"}"#);
            /// let body = response.as_json::<serde_json::Value>().await?;
            /// let expected = serde_json::json!({ "hello": "world" });
            /// assert_eq!(body, expected);
            /// # Ok(())
            /// # }
            /// ```
            pub async fn as_json<T: serde::de::DeserializeOwned>(
                &mut self,
            ) -> Result<T, crate::UnderError> {
                let bytes = self.as_bytes().await?;
                serde_json::from_slice(&bytes[..]).map_err(crate::UnderError::JsonDeserialization)
            }

            /// Parses the contents of the body as x-www-form-urlencoded,
            /// deserializaing it into the given value.  This
            /// assumes that the request body is already UTF-8, or a UTF-8 compatible
            /// encoding, and does not check the content-type to make sure.  If that
            /// is a concern, use [`Self::as_bytes`], and handle the conversion
            /// yourself; or, if it's a common occurrance, open a ticket, with your
            /// use-case and a proposed solution.
            ///
            /// # Note
            /// This provides an implicit limit of 3,000,000 bytes. If the body
            /// exceeds this limit, then this function will return an error.
            ///
            /// # Examples
            /// ```rust
            /// # use under::*;
            /// # #[tokio::main] async fn main() -> Result<(), anyhow::Error> {
            /// let mut response = Response::text(r#"hello=world"#);
            /// let body = response.as_form::<serde_json::Value>().await?;
            /// let expected = serde_json::json!({ "hello": "world" });
            /// assert_eq!(body, expected);
            /// # Ok(())
            /// # }
            /// ```
            pub async fn as_form<T: serde::de::DeserializeOwned>(
                &mut self,
            ) -> Result<T, crate::UnderError> {
                let bytes = self.as_bytes().await?;
                serde_urlencoded::from_bytes(&bytes[..])
                    .map_err(crate::UnderError::FormDeserialization)
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
            /// # #[tokio::main] async fn main() -> Result<(), anyhow::Error> {
            /// let mut response = Response::text(r#"{"hello": "world"}"#);
            /// response
            ///     .set_header(http::header::CONTENT_TYPE, "application/json")?;
            /// let body = response.as_sniff::<serde_json::Value>().await?;
            /// let expected = serde_json::json!({ "hello": "world" });
            /// assert_eq!(body, expected);
            /// # Ok(())
            /// # }
            /// ```
            pub async fn as_sniff<T: serde::de::DeserializeOwned>(
                &mut self,
            ) -> Result<T, crate::UnderError> {
                let ctype = self.content_type();
                if Some(mime::APPLICATION_JSON) == ctype {
                    self.as_json().await
                } else if Some(mime::APPLICATION_WWW_FORM_URLENCODED) == ctype {
                    self.as_form().await
                } else {
                    Err(crate::UnderError::InvalidContentType(ctype))
                }
            }
        }
    };
}
