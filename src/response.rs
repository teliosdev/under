use std::convert::TryFrom;

#[derive(Debug)]
pub struct Response(hyper::Response<hyper::Body>);

macro_rules! forward {
    ($(#[$m:meta])* $v:vis fn $name:ident(&self $(, $pn:ident: $pt:ty)*) -> $ret:ty;) => {
        $(#[$m])* $v fn $name(&self $(, $pn: $pt)*) -> $ret {
            (self.0).$name($($pn),*)
        }
    }
}

impl Response {
    pub fn empty_204() -> Self {
        Response::empty_status(hyper::StatusCode::NO_CONTENT)
    }

    pub fn empty_404() -> Self {
        Response::empty_status(hyper::StatusCode::NOT_FOUND)
    }

    pub fn empty_500() -> Self {
        Response::empty_status(hyper::StatusCode::INTERNAL_SERVER_ERROR)
    }

    pub fn permanent_redirect<T>(location: T) -> Result<Self, anyhow::Error>
    where
        hyper::header::HeaderValue: TryFrom<T>,
        <hyper::header::HeaderValue as TryFrom<T>>::Error: Into<http::Error>,
    {
        Ok(Response(
            hyper::Response::builder()
                .status(hyper::StatusCode::PERMANENT_REDIRECT)
                .header(hyper::header::LOCATION, location)
                .body(hyper::Body::empty())?,
        ))
    }

    pub fn empty_status(status: hyper::StatusCode) -> Self {
        Response(
            hyper::Response::builder()
                .status(status)
                .body(hyper::Body::empty())
                .unwrap(),
        )
    }

    pub fn text<V: Into<String>>(body: V) -> Self {
        Response(hyper::Response::builder().body(body.into().into()).unwrap())
    }

    pub fn with_status<S: Into<hyper::StatusCode>>(mut self, status: S) -> Self {
        *self.0.status_mut() = status.into();
        Response(self.0)
    }

    pub(crate) fn into_inner(self) -> hyper::Response<hyper::Body> {
        self.0
    }

    forward! {
        /// Retrieves the status from the response.
        pub fn status(&self) -> hyper::StatusCode;
    }
}

impl From<hyper::Response<hyper::Body>> for Response {
    fn from(hy: hyper::Response<hyper::Body>) -> Self {
        Response(hy)
    }
}
