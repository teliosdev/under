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
#![deny(clippy::correctness)]
#![warn(missing_docs)]
#![warn(missing_debug_implementations)]

#[macro_use]
extern crate async_trait;

mod endpoint;
pub mod endpoints;
mod error;
mod has_body;
mod has_headers;
pub mod middleware;
mod request;
mod response;
mod router;

pub use self::endpoint::Endpoint;
pub use self::error::UnderError;
pub use self::has_body::HasBody;
pub use self::has_headers::HasHeaders;
pub use self::middleware::Middleware;
pub use self::request::fragment::FragmentSelect;
pub use self::request::Request;
pub use self::response::Response;
pub use self::router::{Path, Router};

#[must_use]
#[inline]
/// This creates a new HTTP router.  This is a shortcut for [`Router::default`].
pub fn http() -> Router {
    Router::default()
}
