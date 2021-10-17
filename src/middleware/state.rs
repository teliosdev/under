use std::pin::Pin;

use crate::{Middleware, Request, Response};

use super::Next;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
/// A state value from the state middleware.
///
/// This is used to create new types from the state values for inserting into
/// the [`Request`] extensions.  As such, it is easily dereferencable into the
/// inner type.
pub struct State<T>(pub T);

impl<T> State<T> {
    /// Turns the given state into its inner value, consuming the state.
    ///
    /// # Examples
    /// ```rust
    /// # use under::middleware::State;
    /// let state = State(123u32);
    /// assert_eq!(state.into_inner(), 123u32);
    /// ```
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> std::ops::Deref for State<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for State<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Clone)]
/// The middleware for inserting state into a request.
///
/// This inserts the inner state value into the request every time the
/// middleware is run, before passing on the request down stream.  You can
/// append as many state middlewares as you like, as long as the inner type
/// `T` does not overlap (otherwise, the last value would win).
///
/// This type requires the inner type to be `Clone`, as it must be cloned on
/// every request.  It is recommended to wrap the type in a reference-counting
/// type, like [`std::sync::Arc`], if it is not already in one.
pub struct StateMiddleware<T>(T);

impl<T> StateMiddleware<T> {
    /// Creates an instance of the state middleware with the given value.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// under::http()
    ///     .with(under::middleware::StateMiddleware::new(123u32));
    /// ```
    pub fn new(value: T) -> Self {
        StateMiddleware(value)
    }
}

#[async_trait]
impl<T: Clone + Send + Sync + 'static> Middleware for StateMiddleware<T> {
    async fn apply(
        self: Pin<&Self>,
        mut request: Request,
        next: Next<'_>,
    ) -> Result<Response, anyhow::Error> {
        request.extensions_mut().insert(State(self.0.clone()));
        next.apply(request).await
    }
}

impl<T> std::fmt::Debug for StateMiddleware<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = std::any::type_name::<T>();

        f.debug_tuple("StateMiddleware").field(&name).finish()
    }
}
