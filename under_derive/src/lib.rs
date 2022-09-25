use proc_macro::TokenStream;

mod from_form;

#[proc_macro_derive(FromForm, attributes(form))]
/// Automatically derive `FromForm` from a struct.
///
/// Note that this requires a struct - as x-www-form-urlencoded is a
/// key-value format, there is no way to derive `FromForm` for a tuple or enum.
/// Thus, there must be keys.  However, for tuple structs, the keys can be
/// specified with the `form` attribute.
///
/// The `form` attribute currently accepts these parameters:
///
/// - `rename_all = "value"` - this may only be specified on the whole struct.
///   If it is specified, the given transformation is applied to all field
///   names, if they are not individually renamed.  Valid values are `"snake_case"`,
///   `"camelCase"`, `"PascalCase"`, `"SCREAMING_SNAKE_CASE"`, `"kebab-case"`,
///   `"SCREAMING-KEBAB-CASE"`, `"lowercase"`, and `"uppercase"`.
/// - `rename = "value"` - this may only be specified on a field.  If it is
///   specified, the given name is used instead of the field name.  This is
///   useful for when the field name is not a valid identifier, or when the
///   field name is not the same as the key in the form.
/// - `alias = "value"` - this may only be specified on a field.  If it is
///   specified, the given name is used as an alias for the field.  This is
///   in addition to, not instead of, the field name; if you want instead of,
///   use `rename`.  This can be specified multiple times to add multiple
///   aliases.
/// - `default` - this may only be specified on a field.  If it is specified,
///   the field is optional; if it is not present in the form, the default value
///   is used (through `Default::default`).
/// - `default = "value"` - similar to above, but it uses the function
///    specified by `value` (as a path) to get the default value.  This is
///    incompatible with `optional` and `multiple`.
/// - `optional` - this may only be specified on a field.  If it is specified,
///   the field is optional; if it is not present in the form, the field is
///   skipped.  It is expected that the type of this field is `Option<T>`.
///   This is different from `default` in how the field is handled: `default`
///   will use the default value of the type, while `optional` will skip the
///   field entirely.  This is incompatible with `default` and `multiple`.
/// - `multiple` - this may only be specified on a field.  If it is specified,
///   the field is a multiple field; it is expected that the type of this field
///   implements `FromFieldMultiple` instead of `FromFieldValue`.  It pushes
///   the inner value every time the key is encountered.  This is incompatible
///   with `default` and `optional`.
/// - `parse_with = "value"` - this may only be specified on a field.  If it is
///   specified, the field is parsed with the given function.  The function must
///   be a path to a function that takes a `&str` and returns a `Result<T, E>`,
///   where `T` is the type of the field and `E` is the error type.
pub fn derive_from_form(item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::ItemStruct);
    proc_macro::TokenStream::from(
        self::from_form::from_form(input).unwrap_or_else(|e| e.into_compile_error()),
    )
}
