use std::fmt::Display;
use std::pin::Pin;

use super::{Middleware, Next};
use crate::{Request, Response};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct TraceMiddleware;

impl TraceMiddleware {
    pub const fn new() -> Self {
        TraceMiddleware
    }
}

impl Default for TraceMiddleware {
    fn default() -> Self {
        TraceMiddleware
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
