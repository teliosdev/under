use phf::phf_set;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
use std::num::{
    NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroIsize, NonZeroU128,
    NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8, NonZeroUsize,
};
use unicase::UniCase;

/// A trait for types that can be created from a form.
///
/// This is for parsing from a HTML form, not a URL query string.  Specifically,
/// this parses from the body of a `application/x-www-form-urlencoded` request.
/// Because form-urlencoded is not a strict data format, e.g. it has no nesting
/// structure, this trait is not implemented for `FromFormValue` and instead
/// provides a `from_form` method.  It is not only allowed that the keys are
/// repeated, but expected for a few things - especially arrays.
///
/// It is expected that you use `#[derive(FromForm)]` for any type that must
/// implement this trait.  This will generate a `from_form` method that
/// correctly parses the types from the form.  If you need to implement this
/// trait by hand, you will need to use [`FromFormValue`] to parse each value
/// (and `FromFormMultiple` for arrays).
///
/// # Examples
///
/// ```rust
/// # use under::FromForm;
///
/// #[derive(FromForm)]
/// struct LoginForm {
///    username: String,
///   password: String,
/// }
///
/// let form = LoginForm::from_form([
///    ("username", "Sergio"),
///   ("password", "hunter2"),
/// ].into_iter()).unwrap();
/// ```
#[doc(cfg(feature = "from_form"))]
pub trait FromForm: Sized {
    /// Takes in an iterator of key-values, and returns a `Result<Self,
    /// FromFormError>`.  The iterator is guaranteed to be in the order that the
    /// keys were encountered in the form.  This is important for arrays, which
    /// are expected to be repeated keys.
    fn from_form<'f, I, K, V>(form: I) -> Result<Self, FromFormError>
    where
        I: Iterator<Item = (K, V)>,
        K: AsRef<str> + 'f,
        V: AsRef<str> + 'f;
}

impl<V, S: std::hash::BuildHasher + Default> FromForm for std::collections::HashMap<String, V, S>
where
    V: for<'f> FromFormMultiple<'f>,
    for<'f> <V as FromFormMultiple<'f>>::Item: FromFormValue<'f>,
    for<'f> <<V as FromFormMultiple<'f>>::Item as FromFormValue<'f>>::Error: Into<anyhow::Error>,
{
    fn from_form<'f, I, K, VV>(form: I) -> Result<Self, FromFormError>
    where
        I: Iterator<Item = (K, VV)>,
        K: AsRef<str> + 'f,
        VV: AsRef<str> + 'f,
    {
        let mut map = <std::collections::HashMap<String, V, S> as Default>::default();
        for (key, value) in form {
            let key = key.as_ref().to_string();
            let value = V::Item::from_form_value(value.as_ref()).map_err(|e| {
                FromFormError::InvalidFormat("-", std::any::type_name::<V::Item>(), e.into())
            })?;
            map.entry(key).or_insert_with(V::default).push(value);
        }

        Ok(map)
    }
}

/// A trait for types that can be created from a form.
///
/// This is for parsing from a HTML form, not a URL query string.  Specifically,
/// this parses from the body of a `application/x-www-form-urlencoded` request.
/// This takes in the form value specified in the key-value pair provided to
/// `FromForm`, and returns a `Result<Self, Self::Error>`.  This is similar to
/// `FromStr`, but allows for different parsings.  For example, `bool` parses
/// from `1`, `true`, `on`, and `yes` (with other values defaulting to `false`).
#[doc(cfg(feature = "from_form"))]
pub trait FromFormValue<'f>: Sized {
    /// The error type that can be returned if parsing fails.  This is normally
    /// encapsulated into a [`anyhow::Error`] before being turned into a variant
    /// of the [`FromFormError`].
    type Error;

    /// Converts the given value into a `Self`.
    fn from_form_value(value: &'f str) -> Result<Self, Self::Error>;
}

/// A trait for types that can be created from a form.
///
/// This is used exclusively for arrays.  It is expected that you use the
/// `#[derive(FromForm)]` attribute on your type.  This will generate a
/// `from_form` method that correctly parses the types from the form.  This
/// handles types that expect multiple key-value pairs with the same keys,
/// such as for arrays.  As such, this trait is automatically implemented for
/// any type `T` such that `T: Default + Extend<V> + IntoIterator<Item = V>`.
/// This should cover all cases, and you should not need to implement (or use)
/// this.
#[doc(cfg(feature = "from_form"))]
pub trait FromFormMultiple<'f>: Sized {
    /// The item type that is being collected into `Self`.
    type Item;

    /// Given for a key-value pair, adds the value to `self`.  This also has
    /// the side effect of parsing the item into the correct type (using its
    /// corresponding `FromFormValue` implementation).
    fn push(&mut self, item: Self::Item);

    /// Return the default (empty) collection of `Self`.  This should just be
    /// [`Default::default`].
    fn default() -> Self;
}

impl<'f, V, T> FromFormMultiple<'f> for T
where
    T: Default + Extend<V> + IntoIterator<Item = V>,
{
    type Item = V;

    fn push(&mut self, item: Self::Item) {
        self.extend(Some(item));
    }

    fn default() -> Self {
        Self::default()
    }
}

impl<'f> FromFormValue<'f> for String {
    type Error = std::convert::Infallible;
    fn from_form_value(value: &'f str) -> Result<Self, Self::Error> {
        Ok(value.to_string())
    }
}

impl<'f> FromFormValue<'f> for &'f str {
    type Error = std::convert::Infallible;
    fn from_form_value(value: &'f str) -> Result<Self, Self::Error> {
        Ok(value)
    }
}

impl<'f, T> FromFormValue<'f> for Option<T>
where
    T: FromFormValue<'f>,
{
    type Error = std::convert::Infallible;
    fn from_form_value(value: &'f str) -> Result<Self, Self::Error> {
        Ok(T::from_form_value(value).ok())
    }
}

impl<'f, T> FromFormValue<'f> for Result<T, T::Error>
where
    T: FromFormValue<'f>,
{
    type Error = std::convert::Infallible;
    fn from_form_value(value: &'f str) -> Result<Self, Self::Error> {
        Ok(T::from_form_value(value))
    }
}

static TRUE_VALUES: phf::Set<UniCase<&'static str>> = phf_set!(
    UniCase::ascii("true"),
    UniCase::ascii("on"),
    UniCase::ascii("yes"),
    UniCase::ascii("1"),
);

impl<'f> FromFormValue<'f> for bool {
    type Error = std::convert::Infallible;
    fn from_form_value(value: &'f str) -> Result<Self, Self::Error> {
        Ok(TRUE_VALUES.contains(&UniCase::new(value)))
    }
}

macro_rules! forward_to_parse {
    ($($t:ty),*) => {
        $(
            impl<'f> FromFormValue<'f> for $t {
                type Error = <$t as std::str::FromStr>::Err;
                fn from_form_value(value: &'f str) -> Result<Self, Self::Error> {
                    value.parse()
                }
            }
        )*
    };
}

forward_to_parse! {
    f32, f64, isize, i8, i16, i32, i64, i128, usize, u8, u16, u32, u64, u128,
    IpAddr, Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6, SocketAddr,
    NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI128, NonZeroIsize,
    NonZeroU8, NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU128, NonZeroUsize
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
#[doc(cfg(feature = "from_form"))]
/// The error type for parsing a form.
///
/// This is returned by [`FromForm::from_form`].  You should not need to
/// implement these yourself.
pub enum FromFormError {
    #[error("missing field `{0}`")]
    /// A field was missing from the form.  This is returned when a field is
    /// missing from the form, but is required.  This includes only the
    /// expected field's primary name - i.e., none of its aliases.  However,
    /// if the field is renamed, the value will be the renamed value.
    MissingField(&'static str),
    #[error("could not parse the field `{0}' as `{1}': {2}")]
    /// A field could not be parsed.  This is returned when a field could not
    /// be parsed into the expected type.  This includes the expected
    /// field's primary name - i.e., none of its aliases (however, if the
    /// field is renamed, the value will be the renamed value).  The second
    /// value is the type that was expected.  The third value is the error
    /// returned by the parser.
    InvalidFormat(&'static str, &'static str, #[source] anyhow::Error),
}
