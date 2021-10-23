//! Pre-defined middleware.
//!
//! This module defines a few middlewares that might be useful for a given HTTP
//! application.  Their use should be as simple as this:
//!
//! ```rust
//! # use under::*;
//! # #[tokio::main] async fn main() -> Result<(), anyhow::Error> {
//! let mut http = under::http();
//! http.at("/home").get(under::endpoints::simple(|| {
//!     Response::text("hello, there!")
//! }));
//! # Ok(())
//! # }
//! ```

#[cfg(feature = "cookie")]
mod cookies;
mod state;
mod trace;
#[cfg(feature = "cookie")]
pub use self::cookies::{CookieExt, CookieMiddleware};
pub use self::state::{State, StateMiddleware};
pub use self::trace::TraceMiddleware;
use crate::{Endpoint, Request, Response};
use std::fmt::Debug;
use std::pin::Pin;

#[derive(Copy, Clone, Debug)]
/// The next item(s) in the stack.
///
/// This borrows from the stack itself, and so the lifetime here exceeds the
/// lifetime of the request (but is not `'static`).  This contains a reference
/// to the eventual endpoint, as well as any (remaining) middleware that must
/// happen next.
pub struct Next<'a> {
    middleware: &'a [Pin<Box<dyn Middleware>>],
    endpoint: Pin<&'a dyn Endpoint>,
}

#[async_trait]
/// An HTTP request/response modifier.
///
/// This sits between the raw request and response and the endpoint, allowing
/// custom functions to mutate either before being passed on.  A typical
/// middleware will take the incoming [`Request`], potentially modify it, before
/// calling [`Next::apply`] with the modified request; then, take the resulting
/// [`Response`], potentially modifying it, before returning.  However, since
/// every layer of the stack is fallible, it must be able to handle errors.
pub trait Middleware: Debug + Send + Sync + 'static {
    #[must_use]
    /// Handles the given request, returning a response.  The next parameter
    /// contains the information on how to process everything after the current
    /// middleware, i.e. generating a response from the endpoint.
    async fn apply(
        self: Pin<&Self>,
        request: Request,
        next: Next<'_>,
    ) -> Result<Response, anyhow::Error>;
}

impl<'a> Next<'a> {
    pub(crate) fn new(
        middleware: &'a [Pin<Box<dyn Middleware>>],
        endpoint: Pin<&'a dyn Endpoint>,
    ) -> Self {
        Next {
            middleware,
            endpoint,
        }
    }

    /// This causes all of the remaining middleware and endpoint to be run,
    /// from this point; i.e., if there is any remaining middleware, execute
    /// that (passing in a modified version of this struct); otherwise, execute
    /// the endpoint.
    ///
    /// It is valid behavior to not call this function; not calling this
    /// function means interrupting the stack, and none of the remaining
    /// middleware nor endpoints will be run.  This could be useful for e.g.
    /// requiring authentication, or
    pub async fn apply(self, request: Request) -> Result<Response, anyhow::Error> {
        if let Some((current, next)) = self.middleware.split_first() {
            let new = Next {
                middleware: next,
                endpoint: self.endpoint,
            };
            current.as_ref().apply(request, new).await
        } else {
            self.endpoint.apply(request).await
        }
    }
}
