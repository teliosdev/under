use self::reader::StreamReader;
use tokio::io::{AsyncReadExt, AsyncWrite, Take};

/// The data stream of a body.
///
/// This should be used to read and write data to the body.  There are always
/// implicit limits to streaming data, the only difference is whether or not
/// your code is prepared to handle that limit.
///
/// This allows us to perform operations on a request/response body without
/// having to worry about data limits.
#[derive(Debug)]
#[must_use = "this consumes the body of the request regardless of whether it is used"]
pub struct DataStream {
    /// The underlying stream.
    stream: Take<StreamReader>,
}

#[derive(Debug, Copy, Clone)]
/// Information about a data transfer.  This is the result of
/// [`DataStream::transfer`], and provides information about the state of the
/// stream after the transfer.
pub struct DataTransfer {
    /// The number of bytes that were transferred.  This may be less than the
    /// number of bytes requested if the stream ended.
    pub count: u64,
    /// Whether or not the stream ended before or as a result of the transfer,
    /// not including the limit - if the limit was reached, and there was still
    /// pending data, this will be `false`.
    pub complete: bool,
}

impl DataStream {
    /// Create a new data stream from a hyper body.
    pub(crate) fn new(body: hyper::Body, limit: u64) -> Self {
        Self {
            stream: StreamReader::from(body).take(limit),
        }
    }

    // note: this is destructive on the stream, so it should only be used once.
    async fn limit_exceeded(&mut self) -> std::io::Result<bool> {
        Ok(!self.stream.get_mut().is_done().await?)
    }

    /// Read data from the stream.
    ///
    /// This streams from the body into the provided writer, and returns the
    /// number of bytes read and whether or not the stream is complete.
    pub async fn into<W: AsyncWrite + Unpin>(
        mut self,
        writer: &mut W,
    ) -> std::io::Result<DataTransfer> {
        let written = tokio::io::copy(&mut self.stream, writer).await?;
        let complete = !self.limit_exceeded().await?;
        Ok(DataTransfer::new(written, complete))
    }

    /// Dispose of the stream into the void.  This is needed to ensure that the
    /// stream is fully consumed.
    ///
    /// This is a no-op if the stream is already complete.
    pub async fn dispose(mut self) -> std::io::Result<()> {
        if !self.limit_exceeded().await? {
            let mut buf = [0u8; 1024];
            while self.stream.read(&mut buf).await? != 0 {}
        }

        if self.limit_exceeded().await? {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "stream not fully consumed",
            ));
        } else {
            Ok(())
        }
    }

    /// Read data from the stream into a byte array.
    ///
    /// This streams from the body into the provided buffer, and returns the
    /// resulting buffer.  If the body of the request is too large to fit into
    /// the limit of the buffer, then an error is returned.
    pub async fn into_bytes(self) -> std::io::Result<Vec<u8>> {
        let mut buf = Vec::new();
        let transfer = self.into(&mut buf).await?;

        if !transfer.complete {
            Err(std::io::Error::new(
                std::io::ErrorKind::OutOfMemory,
                anyhow::Error::msg("body too large"),
            ))
        } else {
            Ok(buf)
        }
    }

    /// Read data from the stream into a string.
    ///
    /// This streams from the body into the provided buffer, and returns the
    /// resulting buffer.  If the body of the request is too large to fit into
    /// the limit of the buffer, then an error is returned.
    pub async fn into_text(self) -> std::io::Result<String> {
        let bytes = self.into_bytes().await?;
        String::from_utf8(bytes).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                anyhow::Error::msg("stream did not contain valid UTF-8"),
            )
        })
    }

    /// Parses the contents of the body as JSON, deserializing it into the
    /// given value.  JSON has strict limits on the bytes/characters allowed
    /// for serialization/deserialization, so the charset should not matter.
    ///
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// # #[tokio::main] async fn main() -> Result<(), anyhow::Error> {
    /// let stream = DataStream::from(r#"{"hello": "world"}"#);
    /// dbg!(&stream);
    /// let body = stream.into_json::<serde_json::Value>().await?;
    /// let expected = serde_json::json!({ "hello": "world" });
    /// assert_eq!(body, expected);
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "json")]
    #[doc(cfg(feature = "json"))]
    pub async fn into_json<T: serde::de::DeserializeOwned>(self) -> Result<T, crate::UnderError> {
        let bytes = self
            .into_bytes()
            .await
            .map_err(crate::UnderError::ReadBody)?;
        serde_json::from_slice(&bytes[..]).map_err(crate::UnderError::JsonDeserialization)
    }

    /// Parses the contents of the body as CBOR, deserializing it into the
    /// given value.  CBOR has strict limits on the bytes/characters allowed
    /// for serialization/deserialization, so the charset should not matter.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// # use ciborium::cbor;
    /// # #[tokio::main] async fn main() -> Result<(), anyhow::Error> {
    /// let stream = DataStream::from(&[0xA1, 0x65, 0x68, 0x65, 0x6C, 0x6C, 0x6F, 0x65, 0x77, 0x6F, 0x72, 0x6C, 0x64][..]);
    /// let body = stream.into_cbor::<ciborium::value::Value>().await?;
    /// let expected = cbor!({ "hello" => "world" })?;
    /// assert_eq!(body, expected);
    /// # Ok(())
    /// # }
    #[cfg(feature = "cbor")]
    #[doc(cfg(feature = "cbor"))]
    pub async fn into_cbor<T: serde::de::DeserializeOwned>(self) -> Result<T, crate::UnderError> {
        let bytes = self
            .into_bytes()
            .await
            .map_err(crate::UnderError::ReadBody)?;
        ciborium::de::from_reader(&bytes[..])
            .map_err(|e| crate::UnderError::CborDeserialization(e.into()))
    }

    /// Parses the contents of the body as Msgpack, deserializing it into the
    /// given value.  Msgpack has strict limits on the bytes/characters allowed
    /// for serialization/deserialization, so the charset should not matter.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// # use rmp_serde::Deserializer;
    /// # #[tokio::main] async fn main() -> Result<(), anyhow::Error> {
    /// let stream = DataStream::from(&[0x81, 0xA5, 0x68, 0x65, 0x6C, 0x6C, 0x6F, 0xA5, 0x77, 0x6F, 0x72, 0x6C, 0x64][..]);
    /// let body = stream.into_msgpack::<serde_json::Value>().await?;
    /// let expected = serde_json::json!({ "hello": "world" });
    /// assert_eq!(body, expected);
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "msgpack")]
    #[doc(cfg(feature = "msgpack"))]
    pub async fn into_msgpack<T: serde::de::DeserializeOwned>(
        self,
    ) -> Result<T, crate::UnderError> {
        let bytes = self
            .into_bytes()
            .await
            .map_err(crate::UnderError::ReadBody)?;
        rmp_serde::from_slice(&bytes[..]).map_err(crate::UnderError::MsgpackDeserialization)
    }

    /// Parses the contents of the body as x-www-form-urlencoded,
    /// deserializing it into the given value.  This
    /// assumes that the request body is already UTF-8, or a UTF-8 compatible
    /// encoding, and does not check the content-type to make sure.  If that
    /// is a concern, use [`Self::as_bytes`], and handle the conversion
    /// yourself; or, if it's a common occurrence, open a ticket, with your
    /// use-case and a proposed solution.
    ///
    /// # Note
    /// This provides an implicit limit of 3,000,000 bytes. If the body
    /// exceeds this limit, then this function will return an error.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// # use std::collections::HashMap;
    /// # #[tokio::main] async fn main() -> Result<(), anyhow::Error> {
    /// let stream = DataStream::from(r#"hello=world"#);
    /// let body = stream.into_form::<HashMap<String, Vec<String>>>().await?;
    /// assert_eq!(&body["hello"][..], &["world".to_string()]);
    /// assert_eq!(body.len(), 1);
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "from_form")]
    #[doc(cfg(feature = "from_form"))]
    pub async fn into_form<T: crate::from_form::FromForm>(self) -> Result<T, crate::UnderError> {
        let bytes = self
            .into_bytes()
            .await
            .map_err(crate::UnderError::ReadBody)?;
        let items = form_urlencoded::parse(&bytes);
        T::from_form(items).map_err(crate::UnderError::FormDeserialization)
    }
}

impl<T> From<T> for DataStream
where
    T: Into<hyper::Body>,
{
    fn from(body: T) -> Self {
        use hyper::body::HttpBody;
        let body = body.into();
        let size_hint = body.size_hint();
        let limit = size_hint
            .upper()
            .unwrap_or_else(|| size_hint.lower())
            .min(3_000_000)
            + 1;
        Self::new(body.into(), limit)
    }
}

impl DataTransfer {
    fn new(count: u64, complete: bool) -> Self {
        Self { count, complete }
    }
}

mod reader {
    use futures::Stream;
    use std::io;
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use tokio::io::{AsyncRead, ReadBuf};

    #[derive(Debug)]
    enum State {
        Pending,
        Done,
        Ready(hyper::body::Bytes, usize),
        // Partial(Cursor<hyper::body::Bytes>),
    }

    #[derive(Debug)]
    #[pin_project::pin_project]
    pub struct StreamReader {
        #[pin]
        inner: hyper::Body,
        state: State,
    }

    impl StreamReader {
        pub(super) async fn is_done(&mut self) -> Result<bool, io::Error> {
            let mut this = Pin::new(self);
            std::future::poll_fn(move |x| this.as_mut().poll_done(x)).await
        }

        /// Polls to determine if this stream is done.  For the `Done` state,
        /// this is easy - we are definitely done.  For the `Partial` state,
        fn poll_done(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<bool>> {
            loop {
                match self.as_mut().state {
                    State::Pending => {
                        match futures::ready!(self.as_mut().project().inner.poll_next(cx)) {
                            Some(Ok(v)) => {
                                let c = v.len() == 0;
                                self.as_mut().state = State::Ready(v, 0);
                                return Poll::Ready(Ok(c));
                            }
                            Some(Err(e)) => {
                                return Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, e)))
                            }
                            None => {
                                self.as_mut().state = State::Done;
                                return Poll::Ready(Ok(true));
                            }
                        }
                    }
                    State::Ready(ref c, ref mut start) => {
                        if *start < c.len() {
                            return Poll::Ready(Ok(false));
                        } else {
                            self.as_mut().state = State::Pending;
                            // this will cause a loop
                        }
                    }
                    State::Done => return Poll::Ready(Ok(true)),
                }
            }
        }
    }

    impl<'r> From<hyper::Body> for StreamReader {
        fn from(body: hyper::Body) -> Self {
            Self {
                inner: body,
                state: State::Pending,
            }
        }
    }

    impl AsyncRead for StreamReader {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            loop {
                self.state = match self.state {
                    State::Ready(ref b, ref mut start) => {
                        let len = std::cmp::min(buf.remaining(), b.len() - *start);
                        buf.put_slice(&b[*start..*start + len]);
                        *start += len;

                        if b.len() >= *start {
                            self.state = State::Pending;
                        }
                        return Poll::Ready(Ok(()));
                    }
                    State::Pending => {
                        match futures::ready!(self.as_mut().project().inner.poll_next(cx)) {
                            Some(Err(e)) => {
                                self.state = State::Done;
                                return Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, e)));
                            }
                            Some(Ok(b)) => State::Ready(b, 0),
                            None => {
                                self.state = State::Done;
                                return Poll::Ready(Ok(()));
                            }
                        }
                    }
                    State::Done => return Poll::Ready(Ok(())),
                }
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use hyper::Body;
        use tokio::io::AsyncReadExt;

        #[tokio::test]
        async fn test_stream_reader() {
            let body = Body::from(&[1, 2, 3, 4, 5][..]);
            let mut stream = StreamReader::from(body);
            let mut buf = [0u8; 5];
            let b = stream.read(&mut buf).await.unwrap();
            assert_eq!(b, 5);
            assert_eq!(buf, [1, 2, 3, 4, 5]);
        }

        #[tokio::test]
        async fn test_empty_stream_reader() {
            let body = Body::empty();
            let mut stream = StreamReader::from(body);
            let mut buf = [0u8; 5];
            let b = stream.read(&mut buf).await.unwrap();
            assert_eq!(b, 0);
        }

        #[tokio::test]
        async fn test_large_body_read() {
            let body = Body::from(vec![1u8; 100_000]);
            let mut stream = StreamReader::from(body);
            let mut out = Vec::new();
            let b = stream.read_to_end(&mut out).await.unwrap();

            assert_eq!(b, 100_000);
        }
    }
}
