macro_rules! has_extensions {
    ($ty:ty) => {
        impl $ty {
            /// Returns state information provided by the
            /// [`crate::middleware::StateMiddleware`] middleware.  This is a
            /// shortcut to retrieving the [`crate::middleware::State`]
            /// extension from the request.
            ///
            /// # Examples
            /// ```rust
            /// # use under::*;
            /// use under::middleware::State;
            /// let mut request = Request::get("/").unwrap();
            /// request.extensions_mut().insert(State(123u32));
            /// assert_eq!(request.state::<u32>(), Some(&123u32));
            /// ```
            pub fn state<T: Send + Sync + 'static>(&self) -> Option<&T> {
                self.ext::<crate::middleware::State<T>>().map(|v| &v.0)
            }

            /// Retrieves a specific extension from the extensions map.  This is
            /// the same as calling [`Self::extensions`].`get` wit the given
            /// type parameter.
            ///
            /// # Examples
            /// ```rust
            /// # use under::*;
            /// let mut request = Request::get("/").unwrap();
            /// assert_eq!(request.ext::<u32>(), None);
            /// ```
            pub fn ext<T: Send + Sync + 'static>(&self) -> Option<&T> {
                self.extensions().get::<T>()
            }

            /// Retrieves a mutable reference to the specific extension from the
            /// extensions map.  This is the same as calling
            /// [`Self::extensions_mut`].`get_mut` with the given type
            /// parameter.
            ///
            /// # Examples
            /// ```rust
            /// # use under::*;
            /// let mut request = Request::get("/").unwrap();
            /// assert_eq!(request.ext_mut::<u32>(), None);
            /// ```
            pub fn ext_mut<T: Send + Sync + 'static>(&mut self) -> Option<&mut T> {
                self.extensions_mut().get_mut::<T>()
            }

            /// Sets the value of the specific extension in the extensions map.
            /// This is the same as calling [`Self::extensions_mut`].`insert`
            /// with the given parameter.
            ///
            /// # Examples
            /// ```rust
            /// # use under::*;
            /// let mut request = Request::get("/").unwrap();
            /// request.set_ext(123u32);
            /// assert_eq!(request.ext::<u32>(), Some(&123u32));
            /// ```
            pub fn set_ext<T: Send + Sync + 'static>(&mut self, value: T) -> &mut Self {
                self.extensions_mut().insert(value);
                self
            }

            /// Sets the value of the specific extension in the extensions map,
            /// consuming `self`, and then returning the new value.  This is
            /// the same as calling [`Self::set_ext`], but it consumes `self`.
            ///
            /// # Examples
            /// ```rust
            /// # use under::*;
            /// let request = Request::get("/").unwrap();
            /// let request = request.with_ext(123u32);
            /// assert_eq!(request.ext::<u32>(), Some(&123u32));
            /// ```
            pub fn with_ext<T: Send + Sync + 'static>(mut self, value: T) -> Self {
                self.set_ext(value);
                self
            }

            /// Removes the specific extension from the extensions map.  This is
            /// the same as calling [`Self::extensions_mut`].`remove` with the
            /// given type parameter.
            ///
            /// # Examples
            /// ```rust
            /// # use under::*;
            /// let mut request = Request::get("/").unwrap()
            ///     .with_ext(123u32);
            /// assert_eq!(request.ext::<u32>(), Some(&123u32));
            /// request.remove_ext::<u32>();
            /// assert_eq!(request.ext::<u32>(), None);
            /// ```
            pub fn remove_ext<T: Send + Sync + 'static>(&mut self) -> Option<T> {
                self.extensions_mut().remove::<T>()
            }

            /// Removes the specific extension from the extensions map,
            /// consuming `self`, and then returning the removed value.  This
            /// is the same as calling [`Self::remove_ext`], but it consumes
            /// `self`.
            ///
            /// # Examples
            /// ```rust
            /// # use under::*;
            /// let request = Request::get("/").unwrap()
            ///     .with_ext(123u32);
            /// assert_eq!(request.ext::<u32>(), Some(&123u32));
            /// let request = request.without_ext::<u32>();
            /// assert_eq!(request.ext::<u32>(), None);
            /// ```
            pub fn without_ext<T: Send + Sync + 'static>(mut self) -> Self {
                self.remove_ext::<T>();
                self
            }
        }
    };
}
