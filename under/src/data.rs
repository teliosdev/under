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
#[must_use = "this consumes the body of the request regardless of whether it is used"]
pub struct DataStream {
    /// The underlying stream.
    stream: Take<StreamReader>,
}

#[derive(Debug, Copy, Clone)]
pub struct DataTransfer {
    pub count: u64,
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
        if self.stream.limit() == 0 {
            return Ok(true);
        }

        // If we've reached the limit, we need to check if we will exceed it.
        self.stream.set_limit(1);
        let mut buf = [0u8; 1];
        // if we've read _any_ bytes into the buffer, then we exceeded the
        // limit.  Oops!
        Ok(self.stream.read(&mut buf).await? != 0)
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
}

impl DataTransfer {
    fn new(count: u64, complete: bool) -> Self {
        Self { count, complete }
    }
}

impl std::fmt::Debug for DataStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DataStream").field("stream", &()).finish()
    }
}

mod reader {
    use std::io;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    use bytes::Buf;
    use futures::Stream;
    use std::io::Cursor;
    use tokio::io::{AsyncRead, ReadBuf};

    enum State {
        Pending,
        Done,
        Partial(Cursor<hyper::body::Bytes>),
    }

    #[pin_project::pin_project]
    pub struct StreamReader {
        #[pin]
        inner: hyper::Body,
        state: State,
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
                    State::Pending => {
                        match futures::ready!(Pin::new(&mut self.inner).poll_next(cx)) {
                            Some(Err(e)) => {
                                return Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, e)))
                            }
                            Some(Ok(b)) => State::Partial(Cursor::new(b)),
                            None => State::Done,
                        }
                    }
                    State::Done => return Poll::Ready(Ok(())),
                    State::Partial(ref mut cursor) => {
                        let remaining = cursor.remaining();
                        match futures::ready!(Pin::new(cursor).poll_read(cx, buf)) {
                            Ok(()) if remaining == buf.remaining() => State::Pending,
                            result => return Poll::Ready(result),
                        }
                    }
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
    }
}
