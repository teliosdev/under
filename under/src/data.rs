use futures::stream::MapErr;
use futures::TryStreamExt;
use tokio::io::{AsyncReadExt, AsyncWrite, Take};
use tokio_util::io::StreamReader;

use crate::UnderError;

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
    stream: Take<StreamReader<HttpStream, hyper::body::Bytes>>,
}

type HttpStream = MapErr<hyper::Body, fn(hyper::Error) -> std::io::Error>;

#[derive(Debug, Copy, Clone)]
/// Information about a data transfer.  This is the result of
/// [`DataStream::into`], and provides information about the state of the
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
            stream: StreamReader::new(body.map_err(map_hyper_error as fn(_) -> _)).take(limit + 1),
        }
    }

    // note: this is destructive on the stream, so it should only be used once.
    fn limit_exceeded(&mut self) -> bool {
        self.stream.limit() <= 1
    }

    /// Read data from the stream.
    ///
    /// This streams from the body into the provided writer, and returns the
    /// number of bytes read and whether or not the stream is complete.
    ///
    /// # Errors
    /// This returns an error if the underlying stream cannot be written to the
    /// given writer.  It does not return an error if the stream is incomplete,
    /// as that is expected to be handled by the caller.
    pub async fn into<W: AsyncWrite + Unpin>(
        mut self,
        writer: &mut W,
    ) -> Result<DataTransfer, UnderError> {
        let written = tokio::io::copy(&mut self.stream, writer)
            .await
            .map_err(UnderError::ReadBody)?;
        let complete = !self.limit_exceeded();
        Ok(DataTransfer::new(written, complete))
    }

    /// Read data from the stream into a byte array.
    ///
    /// This streams from the body into the provided buffer, and returns the
    /// resulting buffer.  If the body of the request is too large to fit into
    /// the limit of the buffer, then an error is returned.
    ///
    /// # Errors
    /// This returns an error if the underlying stream cannot be written to a
    /// buffer, or if the stream is incomplete.
    pub async fn into_bytes(self) -> Result<Vec<u8>, UnderError> {
        let mut buf = Vec::new();
        let transfer = self.into(&mut buf).await?;

        if transfer.complete {
            Ok(buf)
        } else {
            Err(UnderError::PayloadTooLarge(anyhow::anyhow!(
                "body too large"
            )))
        }
    }

    /// Read data from the stream into a string.
    ///
    /// This streams from the body into the provided buffer, and returns the
    /// resulting buffer.  If the body of the request is too large to fit into
    /// the limit of the buffer, then an error is returned.
    ///
    /// # Errors
    /// Errors for the same reason as [`DataStream::into_bytes`], and also
    /// returns an error if the body cannot be converted to a UTF-8 string.
    pub async fn into_text(self) -> Result<String, UnderError> {
        let bytes = self.into_bytes().await?;
        String::from_utf8(bytes).map_err(UnderError::TextDeserialization)
    }

    /// Parses the contents of the body as JSON, deserializing it into the
    /// given value.  JSON has strict limits on the bytes/characters allowed
    /// for serialization/deserialization, so the charset should not matter.
    ///
    /// # Errors
    /// Errors for the same reason as [`DataStream::into_bytes`], and also
    /// returns an error if the body cannot be converted to a JSON value.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// # #[tokio::main] async fn main() -> Result<(), anyhow::Error> {
    /// let stream = DataStream::from(r#"{"hello": "world"}"#);
    /// dbg!(&stream);
    /// let body = stream.into_json::<serde_json::Value>().await.unwrap();
    /// let expected = serde_json::json!({ "hello": "world" });
    /// assert_eq!(body, expected);
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "json")]
    #[cfg_attr(nightly, doc(cfg(feature = "json")))]
    pub async fn into_json<T: serde::de::DeserializeOwned>(self) -> Result<T, UnderError> {
        let bytes = self.into_bytes().await?;
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
    #[cfg_attr(nightly, doc(cfg(feature = "cbor")))]
    pub async fn into_cbor<T: serde::de::DeserializeOwned>(self) -> Result<T, UnderError> {
        let bytes = self.into_bytes().await?;
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
    #[cfg_attr(nightly, doc(cfg(feature = "msgpack")))]
    pub async fn into_msgpack<T: serde::de::DeserializeOwned>(self) -> Result<T, UnderError> {
        let bytes = self.into_bytes().await?;
        rmp_serde::from_slice(&bytes[..]).map_err(crate::UnderError::MsgpackDeserialization)
    }

    /// Parses the contents of the body as x-www-form-urlencoded,
    /// deserializing it into the given value.  This
    /// assumes that the request body is already UTF-8, or a UTF-8 compatible
    /// encoding, and does not check the content-type to make sure.  If that
    /// is a concern, use [`Self::into_bytes`], and handle the conversion
    /// yourself; or, if it's a common occurrence, open a ticket, with your
    /// use-case and a proposed solution.
    ///
    /// # Errors
    /// Errors for the same reason as [`DataStream::into_bytes`], and also
    /// returns an error if the body cannot be converted to a form value.
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
    #[cfg_attr(nightly, doc(cfg(feature = "from_form")))]
    pub async fn into_form<T: crate::from_form::FromForm>(self) -> Result<T, UnderError> {
        let bytes = self.into_bytes().await?;
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
        Self::new(body, limit)
    }
}

impl DataTransfer {
    fn new(count: u64, complete: bool) -> Self {
        Self { count, complete }
    }
}

fn map_hyper_error(e: hyper::Error) -> std::io::Error {
    if e.is_closed() || e.is_incomplete_message() || e.is_canceled() {
        std::io::Error::new(std::io::ErrorKind::UnexpectedEof, e)
    } else {
        std::io::Error::new(std::io::ErrorKind::Other, e)
    }
}
