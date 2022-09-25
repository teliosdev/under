macro_rules! has_headers {
    ($ty:ty) => {
        impl $ty {
            /// Retrieves the given header specified here.
            ///
            /// # Examples
            /// ```rust
            /// # use under::*;
            /// let response = Response::text("hello, world");
            /// let content_type = response.header("Content-Type").unwrap();
            /// assert_eq!(content_type.as_bytes(), b"text/plain; charset=utf-8");
            /// ```
            pub fn header<H: http::header::AsHeaderName>(
                &self,
                key: H,
            ) -> Option<&http::HeaderValue> {
                self.headers().get(key)
            }

            /// Retrieves all potential values for the given header specified
            /// here.
            pub fn header_all<H: http::header::AsHeaderName>(
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
            pub fn set_header<H, V>(&mut self, key: H, value: V) -> Result<(), http::Error>
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
            pub fn with_header<H, V>(mut self, key: H, value: V) -> Result<Self, http::Error>
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
            pub fn add_header<H, V>(&mut self, key: H, value: V) -> Result<(), http::Error>
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
            pub fn with_add_header<H, V>(mut self, key: H, value: V) -> Result<Self, http::Error>
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
            pub fn content_type(&self) -> Option<mime::Mime> {
                let content_type = self.headers().get(http::header::CONTENT_TYPE)?;
                let content_type = content_type.to_str().ok()?;
                let content_type = content_type.parse::<mime::Mime>().ok()?;
                Some(content_type)
            }
        }
    };
}
