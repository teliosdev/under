use super::Endpoint;
use crate::{Request, Response};
use anyhow::Error;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::AsyncRead;
use tokio::stream::Stream;

#[derive(Debug, Clone)]
pub(super) struct DirEndpoint {
    base: PathBuf,
}

impl DirEndpoint {
    pub(super) fn new<P: Into<PathBuf>>(path: P) -> Self {
        DirEndpoint { base: path.into() }
    }
}

impl<D> Endpoint<D> for DirEndpoint
where
    D: Send + Sync + 'static,
{
    fn apply<'s, 'a>(
        &'s self,
        request: Request<D>,
    ) -> Pin<Box<dyn Future<Output = Result<Response, Error>> + Send + 'a>>
    where
        's: 'a,
        Self: 'a,
    {
        Box::pin(async move {
            let uri_path = request.uri().path();
            let result = match resolve_path(request.fragment_index::<String>(1), &self.base) {
                Some(path) => resolve_file(path, &uri_path).await,
                None => Ok(Response::empty_404()),
            };
            result
        })
    }
}

lazy_static::lazy_static! {
    static ref DOUBLE_DOT: regex::Regex = regex::Regex::new("/.+/\\.\\.").unwrap();
}

fn resolve_path(param: Option<String>, base: &Path) -> Option<PathBuf> {
    log::trace!("resolve_path({:?}, {:?})", param, base);
    let param = param?;

    let replace = DOUBLE_DOT.replace_all(&param, "/");

    log::trace!("resolve_path.replace={:?}", replace);
    let request = replace
        .split('/')
        .skip_while(|p| p.is_empty() || *p == "..")
        .filter(|p| !p.is_empty() && *p != ".");
    let mut buffer = base.to_path_buf();
    request.for_each(|p| {
        log::trace!("resolve_path.push({:?})", p);
        buffer.push(p);
    });
    log::trace!("resolve_path={:?}", buffer);
    Some(buffer)
}

fn tap<It>(pos: &'static str, v: impl Iterator<Item = It>) -> impl Iterator<Item = It>
where
    It: std::fmt::Debug,
{
    let result = v.collect::<Vec<_>>();
    log::trace!("tap({:?})={:?}", pos, result);
    result.into_iter()
}

async fn resolve_file(mut path: PathBuf, request: &str) -> Result<Response, Error> {
    match tokio::fs::metadata(&path).await {
        Ok(meta) if meta.is_dir() && !request.ends_with('/') => {
            return Response::permanent_redirect(format!("{}/", request));
        }
        Ok(meta) if meta.is_dir() => {
            path.push("index.html");
            if !tokio::fs::metadata(&path)
                .await
                .map(|m| m.is_file())
                .unwrap_or(false)
            {
                return Ok(Response::empty_404());
            }
        }
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Response::empty_404()),
        Err(e) => return Err(e.into()),
    }

    load_file(tokio::fs::File::open(&path).await?, &path)
}

fn load_file(file: tokio::fs::File, path: &Path) -> Result<Response, Error> {
    let mime_type = mime_guess::MimeGuess::from_path(&path).first_or_octet_stream();
    hyper::Response::builder()
        .header(http::header::CONTENT_TYPE, mime_type.to_string())
        .status(hyper::StatusCode::OK)
        .body(hyper::Body::wrap_stream(StreamRead::new(file)))
        .map(Response::from)
        .map_err(Error::from)
}

pub struct StreamRead {
    file: tokio::fs::File,
    buffer: Box<[u8; 4096]>,
}

impl StreamRead {
    /// Create a new stream from the given file.
    pub fn new(file: tokio::fs::File) -> Self {
        StreamRead {
            file,
            buffer: Box::new([0; 4096]),
        }
    }
}

impl Stream for StreamRead {
    type Item = Result<bytes::Bytes, std::io::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let StreamRead { file, buffer } = &mut *self;
        match Pin::new(file).poll_read(cx, &mut buffer[..]) {
            Poll::Ready(Ok(0)) => Poll::Ready(None),
            Poll::Ready(Ok(size)) => Poll::Ready(Some(Ok(buffer[..size].to_owned().into()))),
            Poll::Ready(Err(e)) => Poll::Ready(Some(Err(e))),
            Poll::Pending => Poll::Pending,
        }
    }
}
