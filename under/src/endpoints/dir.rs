use super::Endpoint;
use crate::{Request, Response};
use anyhow::Error;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use tokio_util::io::ReaderStream;

#[derive(Debug, Clone)]
pub(super) struct DirEndpoint {
    base: PathBuf,
}

impl DirEndpoint {
    pub(super) fn new<P: Into<PathBuf>>(path: P) -> Self {
        DirEndpoint { base: path.into() }
    }
}

#[async_trait]
impl Endpoint for DirEndpoint {
    async fn apply(self: Pin<&Self>, request: Request) -> Result<Response, Error> {
        let uri_path = request.uri().path();
        match resolve_path(request.fragment::<String, _>(1), &self.base) {
            Some(path) => resolve_file(path, uri_path).await,
            None => Ok(Response::empty_404()),
        }
    }
}

fn resolve_path(param: Option<String>, base: &Path) -> Option<PathBuf> {
    let param = param?;

    let split = param.split('/');
    let is_invalid = split.clone().any(|v| v == ".." || v.contains('\\'));

    if is_invalid {
        return None;
    }

    let request = split.filter(|p| !p.is_empty() && *p != ".");
    let mut buffer = base.to_path_buf();
    request.for_each(|p| buffer.push(p));
    Some(buffer)
}

async fn resolve_file(mut path: PathBuf, request: &str) -> Result<Response, Error> {
    match tokio::fs::metadata(&path).await {
        Ok(meta) if meta.is_dir() && !request.ends_with('/') => {
            return Response::permanent_redirect(format!("{request}/")).map_err(Error::from);
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
    let mime_type = mime_guess::MimeGuess::from_path(path).first_or_octet_stream();
    hyper::Response::builder()
        .header(http::header::CONTENT_TYPE, mime_type.to_string())
        .status(hyper::StatusCode::OK)
        .body(hyper::Body::wrap_stream(ReaderStream::new(file)))
        .map(Response::from)
        .map_err(Error::from)
}
