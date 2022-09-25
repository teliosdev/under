//! Under is a batteries-included async HTTP framework built for easy
//! development.  Under is based on Tokio, an async runtime.  Under is meant to
//! take the headache out of developing HTTP servers, while still being fairly
//! performant.
//!
//! # Getting Started
//! To get started, just add under and tokio to your `Cargo.toml`:
//!
//! ```toml
//! under = "0.1.0"
//! tokio = { version = "1.12.0", features = ["full"] } # or whatever the latest version is
//! ```
//!
//! # Examples
//! ```rust,no_run
//! async fn hello_world(_: under::Request) -> Result<under::Response, anyhow::Error> {
//!     Ok(under::Response::text("hello, world!"))
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), anyhow::Error> {
//!     let mut http = under::http();
//!     http.at("/").get(hello_world);
//!     http.listen("0.0.0.0:8080").await?;
//!     Ok(())
//! }
//! ```
#![warn(missing_debug_implementations, missing_docs, rust_2018_idioms)]
#![deny(clippy::correctness, unused_must_use)]
#![feature(doc_cfg)]

#[macro_use]
extern crate async_trait;

mod endpoint;
pub mod endpoints;
mod error;
#[macro_use]
mod has_body;
#[macro_use]
mod has_headers;
#[macro_use]
mod has_extensions;
mod data;
#[cfg(feature = "from_form")]
#[doc(hidden)]
pub mod from_form;
pub mod middleware;
mod request;
mod response;
mod router;
#[cfg(feature = "sse")]
#[doc(cfg(feature = "sse"))]
pub mod sse;

#[cfg(feature = "cookie")]
#[doc(cfg(feature = "cookie"))]
pub use cookie::{Cookie, CookieBuilder, CookieJar};

#[cfg(feature = "from_form")]
#[doc(cfg(feature = "from_form"))]
pub use from_form::{FromForm, FromFormError, FromFormMultiple, FromFormValue};

#[cfg(feature = "under_derive")]
#[allow(unused_imports)]
#[macro_use]
extern crate under_derive;
#[cfg(feature = "under_derive")]
pub use under_derive::*;

pub use self::endpoint::Endpoint;
pub use self::error::UnderError;
pub use self::middleware::Middleware;
pub use self::request::fragment::FragmentSelect;
pub use self::request::{RemoteAddress, Request};
pub use self::response::{IntoResponse, Response};
pub use self::router::{Path, Router};

pub use ::http;
pub use hyper::Body;

/// A type alias for [`std::result::Result`].
///
/// The most common use-case for this type is for endpoints, which return this
/// type as a response for a request.
///
/// # Examples
/// ```rust
/// async fn handle(req: under::Request) -> under::Result {
///     return Ok(under::Response::text("hello, world!"));
/// }
///
/// # use under::*;
/// # #[tokio::main] async fn main() -> Result<(), anyhow::Error> {
/// let mut http = under::http();
/// http.at("/").get(handle);
/// # Ok(())
/// # }
pub type Result<R = Response, E = anyhow::Error> = std::result::Result<R, E>;

#[must_use]
#[inline]
/// This creates a new HTTP router.  This is a shortcut for [`Router::default`].
pub fn http() -> Router {
    Router::default()
}
