use crate::router::Route;
use std::str::FromStr;
use std::sync::Arc;

macro_rules! forward {
    () => {};
    (
        $(#[$m:meta])* $v:vis fn $name:ident(&self $(, $pn:ident: $pt:ty)*) -> $ret:ty;
        $($tail:tt)*
    ) => {
        $(#[$m])* $v fn $name(&self $(, $pn: $pt)*) -> $ret {
            (self.0).$name($($pn),*)
        }

        forward! { $($tail)* }
    };

    (
        $(#[$m:meta])* $v:vis fn $name:ident(&mut self $(, $pn:ident: $pt:ty)*) -> $ret:ty;
        $($tail:tt)*
    ) => {
        $(#[$m])* $v fn $name(&mut self $(, $pn: $pt)*) -> $ret {
            (self.0).$name($($pn),*)
        }

        forward! { $($tail)* }
    }
}

pub struct Request(http::Request<hyper::Body>);

lazy_static::lazy_static! {
    static ref EMPTY_REGEX: regex::Regex = regex::Regex::new("").unwrap();
}

impl Request {
    forward! {
        pub fn uri(&self) -> &http::Uri;
        pub fn extensions(&self) -> &http::Extensions;
        pub fn extensions_mut(&mut self) -> &mut http::Extensions;
        pub fn method(&self) -> &http::Method;
    }

    pub fn fragment<I: FromStr>(&self, name: &str) -> Option<I> {
        self.captures()?
            .name(name)
            .map(|m| m.as_str())
            .and_then(|s| s.parse().ok())
    }

    pub fn fragment_index<I: FromStr>(&self, index: usize) -> Option<I> {
        self.captures()?
            .get(index)
            .map(|m| m.as_str())
            .and_then(|s| s.parse().ok())
    }

    fn captures(&self) -> Option<regex::Captures<'_>> {
        self.extensions()
            .get::<Arc<Route>>()?
            .regex
            .captures(self.uri().path())
    }
}

impl From<http::Request<hyper::Body>> for Request {
    fn from(r: http::Request<hyper::Body>) -> Self {
        Request(r)
    }
}
