use crate::router::Route;
use crate::Stack;
use std::str::FromStr;
use std::sync::Arc;

macro_rules! forward {
    ($(#[$m:meta])* $v:vis fn $name:ident(&self $(, $pn:ident: $pt:ty)*) -> $ret:ty;) => {
        $(#[$m])* $v fn $name(&self $(, $pn: $pt)*) -> $ret {
            self.inner.$name($($pn),*)
        }
    }
}

pub struct Request<D> {
    pub(crate) inner: hyper::Request<hyper::Body>,
    pub(crate) route: Option<Arc<Route<D>>>,
    pub(crate) data: Arc<D>,
}

lazy_static::lazy_static! {
    static ref EMPTY_REGEX: regex::Regex = regex::Regex::new("").unwrap();
}

impl<D> Request<D> {
    forward! {
        pub fn uri(&self) -> &http::Uri;
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
        self.route.as_ref()?.regex.captures(self.uri().path())
    }
}
