use std::convert::TryFrom;

#[derive(Debug)]
pub struct Response(http::Response<hyper::Body>);

macro_rules! forward {
    ($(#[$m:meta])* $v:vis fn $name:ident(&self $(, $pn:ident: $pt:ty)*) -> $ret:ty;) => {
        $(#[$m])* $v fn $name(&self $(, $pn: $pt)*) -> $ret {
            (self.0).$name($($pn),*)
        }
    };

    ($(#[$m:meta])* $v:vis fn $name:ident(&mut self $(, $pn:ident: $pt:ty)*) -> $ret:ty;) => {
        $(#[$m])* $v fn $name(&mut self $(, $pn: $pt)*) -> $ret {
            (self.0).$name($($pn),*)
        }
    }
}

impl Response {
    pub fn empty_204() -> Self {
        Response::empty_status(http::StatusCode::NO_CONTENT)
    }

    pub fn empty_404() -> Self {
        Response::empty_status(http::StatusCode::NOT_FOUND)
    }

    pub fn empty_500() -> Self {
        Response::empty_status(http::StatusCode::INTERNAL_SERVER_ERROR)
    }

    pub fn permanent_redirect<T>(location: T) -> Result<Self, anyhow::Error>
    where
        http::header::HeaderValue: TryFrom<T>,
        <http::header::HeaderValue as TryFrom<T>>::Error: Into<http::Error>,
    {
        Ok(Response(
            http::Response::builder()
                .status(http::StatusCode::PERMANENT_REDIRECT)
                .header(http::header::LOCATION, location)
                .body(hyper::Body::empty())?,
        ))
    }

    pub fn empty_status(status: http::StatusCode) -> Self {
        Response(
            http::Response::builder()
                .status(status)
                .body(hyper::Body::empty())
                .unwrap(),
        )
    }

    pub fn text<V: Into<String>>(body: V) -> Self {
        Response(http::Response::builder().body(body.into().into()).unwrap())
    }

    pub fn json<V: serde::Serialize>(body: &V) -> Result<Self, anyhow::Error> {
        let value = serde_json::to_string(body)?;
        Ok(Response(
            http::Response::builder().body(value.into()).unwrap(),
        ))
    }

    pub fn with_status<S: Into<http::StatusCode>>(mut self, status: S) -> Self {
        *self.0.status_mut() = status.into();
        Response(self.0)
    }

    pub(crate) fn into_inner(self) -> http::Response<hyper::Body> {
        self.0
    }

    forward! {
        /// Retrieves the status from the response.
        pub fn status(&self) -> http::StatusCode;
    }
    forward! {
        pub fn extensions(&self) -> &http::Extensions;
    }
    forward! {
        pub fn extensions_mut(&mut self) -> &mut http::Extensions;
    }
}

impl From<http::Response<hyper::Body>> for Response {
    fn from(hy: http::Response<hyper::Body>) -> Self {
        Response(hy)
    }
}

pub trait IntoResponse {
    fn into_response(self) -> Result<Response, anyhow::Error>;
}

impl IntoResponse for Response {
    fn into_response(self) -> Result<Response, anyhow::Error> {
        Ok(self)
    }
}

impl<E> IntoResponse for Result<Response, E>
where
    E: Into<anyhow::Error>,
{
    fn into_response(self) -> Result<Response, anyhow::Error> {
        self.map_err(Into::into)
    }
}

impl IntoResponse for std::convert::Infallible {
    fn into_response(self) -> Result<Response, anyhow::Error> {
        match self {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_response() {
        let response = Response::empty_500();

        assert!(Ok::<_, anyhow::Error>(response).into_response().is_ok());
    }
}
