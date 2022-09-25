use std::fmt::Display;
use std::pin::Pin;

use super::{Middleware, Next};
use crate::{Request, Response};

#[derive(Default, Debug, Clone)]
/// A middleware for tracing HTTP requests.
///
/// This logs (using `log`) each request, as well as how long each request
/// took.  The default log level is `info`.
pub struct TraceMiddleware {
    _v: (),
}

impl TraceMiddleware {
    #[must_use]
    /// Creates a new trace middleware.  This is provided as an alternative
    /// to `Default`.
    pub fn new() -> Self {
        TraceMiddleware::default()
    }
}

#[async_trait]
impl Middleware for TraceMiddleware {
    async fn apply(
        self: Pin<&Self>,
        request: Request,
        next: Next<'_>,
    ) -> Result<Response, anyhow::Error> {
        let method = request.method().clone();
        let path = request.uri().path().to_string();
        log::info!("--> {} {}", method, path);
        let start = std::time::Instant::now();

        let result = next.apply(request).await;
        let elapse = start.elapsed();
        let status = StatusDisplay(&result);

        log::info!(
            "<-- {} {}: {} (in {}ms)",
            method,
            path,
            status,
            elapse.as_millis()
        );

        result
    }
}

struct StatusDisplay<'a>(&'a Result<Response, anyhow::Error>);

impl Display for StatusDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Ok(response) => write!(f, "{}", response.status()),
            Err(_) => write!(f, "(error)"),
        }
    }
}
