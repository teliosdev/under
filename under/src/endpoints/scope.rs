use std::pin::Pin;

#[derive(Default, Debug)]
/// A builder for a [`ScopeEndpoint`].
///
/// This takes in all of the middleware that should operate before the next
/// endpoint, inserting it into an array.  It operates very similarly to the
/// principle of [`crate::Router::with`].
pub struct ScopeEndpointBuilder(Vec<Pin<Box<dyn crate::Middleware>>>);

impl ScopeEndpointBuilder {
    /// Appends middleware to the scope endpoint.  This operates very similarly
    /// to [`crate::Router::with`].
    pub fn with<M: crate::Middleware>(&mut self, middleware: M) -> &mut Self {
        self.0.push(Box::pin(middleware));
        self
    }

    /// Completes the builder, generating a [`ScopeEndpoint`].
    ///
    /// This does leave the builder in a usable state afterwards, resetting it
    /// to the default state.
    pub fn then<E: crate::Endpoint>(&mut self, endpoint: E) -> ScopeEndpoint {
        let endpoint = Box::pin(endpoint);
        let middleware = std::mem::take(&mut self.0);

        ScopeEndpoint {
            middleware,
            endpoint,
        }
    }
}

#[derive(Debug)]
/// The scope endpoint.
///
/// Created from [`ScopeEndpointBuilder`].  See [`super::scope()`] for more
/// information.
pub struct ScopeEndpoint {
    middleware: Vec<Pin<Box<dyn crate::Middleware>>>,
    endpoint: Pin<Box<dyn crate::Endpoint>>,
}

#[async_trait]
impl crate::Endpoint for ScopeEndpoint {
    async fn apply(
        self: Pin<&Self>,
        request: crate::Request,
    ) -> Result<crate::Response, anyhow::Error> {
        let next = crate::middleware::Next::new(&self.middleware[..], self.endpoint.as_ref());
        next.apply(request).await
    }
}
