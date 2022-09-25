use proc_macro2::Span;

pub(super) fn from_form(s: syn::ItemStruct) -> Result<proc_macro2::TokenStream, syn::Error> {
    let name = &s.ident;
    let rename = parse_form_meta(&s)?.unwrap_or(Rename::None);

    let fields = s
        .fields
        .iter()
        .enumerate()
        .map(|(i, f)| FormFieldMeta::from(f, i, rename))
        .collect::<Result<Vec<_>, _>>()?;

    let field_definitions = fields.iter().map(|f| {
        let variable_name = &f.variable_name;
        let field_ty = &f.r#type;
        if f.multiple {
            quote::quote!(let mut #variable_name: #field_ty = <#field_ty as ::std::default::Default>::default();)
        } else {
            quote::quote!(let mut #variable_name: Option<#field_ty> = None;)
        }
    });

    let field_check = fields.iter().map(|f| {
        let variable_name = &f.variable_name;
        let struct_name_s = ident_lit(&f.struct_name.to_string(), f.struct_name.span());
        let form_key = &f.form_key;
        let field_ty = &f.r#type;

        let raw_ty = quote::quote!(::std::any::type_name::<#field_ty>());

        let acceptable_form_keys = f.aliases.iter().map(|a| {
            quote::quote!(#a)
        }).chain(std::iter::once(quote::quote!(#form_key)));

        if f.multiple {
            quote::quote_spanned! {Span::mixed_site()=>
                #(#acceptable_form_keys)|* => {
                    <#field_ty as ::under::from_form::FromFormMultiple>::push(&mut #variable_name, __value.as_ref())
                        .map_err(|e| ::under::from_form::FromFormError::InvalidFormat(#struct_name_s, #raw_ty, e.into()))?;
                }
            }
        } else if let Some(ref parse_with) = f.parse_with {
            quote::quote_spanned! {Span::mixed_site()=>
                #(#acceptable_form_keys)|* => {
                    #variable_name = Some(#parse_with(__value.as_ref()));
                }
            }
        } else {
            quote::quote_spanned! {Span::mixed_site()=>
                #(#acceptable_form_keys)|* => {
                    #variable_name = Some(<#field_ty as ::under::from_form::FromFormValue>::from_form_value(__value.as_ref())
                        .map_err(|e| ::under::from_form::FromFormError::InvalidFormat(#struct_name_s, #raw_ty, e.into()))?);
                }
            }
        }
    });

    let final_assignment = fields.iter().map(|f| {
        let variable_name = &f.variable_name;
        let prefix = if f.field.ident.is_some() {
            let n = &f.field.ident;
            quote::quote!(#n: )
        } else {
            quote::quote!()
        };
        if f.multiple {
            quote::quote!(#prefix #variable_name)
        } else if f.optional {
            quote::quote!(#prefix #variable_name)
        } else {
            match f.default {
                FormFieldDefaultValue::Yes => {
                    quote::quote!(#prefix #variable_name.unwrap_or_default())
                }
                FormFieldDefaultValue::Custom(ref v) => {
                    quote::quote!(#prefix #variable_name.unwrap_or_else(#v))
                }
                FormFieldDefaultValue::No => {
                    let struct_name_s = ident_lit(&f.struct_name.to_string(), f.struct_name.span());
                    quote::quote!(#prefix #variable_name.ok_or_else(|| ::under::from_form::FromFormError::MissingField(#struct_name_s))?)
                }
            }
        }
    });

    let struct_composition = if is_named(&s.fields) {
        quote::quote! {
            #name {
                #(#final_assignment),*
            }
        }
    } else {
        quote::quote! {
            #name (
                #(#final_assignment),*
            )
        }
    };

    Ok(quote::quote_spanned! {Span::mixed_site()=>
        #[automatically_derived]
        impl ::under::from_form::FromForm for #name {
            fn from_form<'f, I, K, V>(__form: I) -> Result<Self, ::under::from_form::FromFormError>
            where
                I: Iterator<Item = (K, V)>,
                K: AsRef<str> + 'f,
                V: AsRef<str> + 'f,

            {
                #( #field_definitions )*

                for (__key, __value) in __form {
                    match __key.as_ref() {
                        #( #field_check )*
                        _ => {}
                    }
                }

                Ok(#struct_composition)
            }
        }
    })
}

fn parse_form_meta(s: &syn::ItemStruct) -> Result<Option<Rename>, syn::Error> {
    let attrs = s
        .attrs
        .iter()
        .filter(|a| a.path.is_ident("form"))
        .map(|a| a.parse_meta())
        .map(|v| {
            v.and_then(|m| match m {
                syn::Meta::List(l) => Ok(l),
                _ => Err(syn::Error::new_spanned(m, "expected #[form(...)]")),
            })
        });
    for list in attrs {
        let list = list?.nested;
        for meta in list {
            match meta {
                syn::NestedMeta::Meta(syn::Meta::NameValue(nv)) => {
                    if nv.path.is_ident("rename_all") {
                        if let syn::Lit::Str(ref s) = nv.lit {
                            return Rename::from_str(&s.value())
                                .map(Some)
                                .map_err(|e| syn::Error::new_spanned(&nv.lit, e));
                        } else {
                            return Err(syn::Error::new_spanned(
                                &nv.lit,
                                "expected string literal",
                            ));
                        }
                    }
                }
                s => {
                    return Err(syn::Error::new_spanned(
                        s,
                        "expected #[form(rename = \"...\")]",
                    ))
                }
            }
        }
    }
    Ok(None)
}

#[derive(Debug)]
struct FormFieldMeta<'f> {
    field: &'f syn::Field,
    r#type: syn::Type,
    variable_name: syn::Ident,
    struct_name: syn::Ident,
    form_key: String,
    aliases: Vec<String>,
    multiple: bool,
    optional: bool,
    default: FormFieldDefaultValue,
    parse_with: Option<syn::ExprPath>,
}

#[derive(Debug)]
enum FormFieldDefaultValue {
    No,
    Yes,
    Custom(syn::ExprPath),
}

impl FormFieldDefaultValue {
    fn has_value(&self) -> bool {
        !matches!(self, FormFieldDefaultValue::No)
    }
}

impl<'f> FormFieldMeta<'f> {
    fn from(
        field: &'f syn::Field,
        i: usize,
        rename: Rename,
    ) -> Result<FormFieldMeta<'f>, syn::Error> {
        let mut name = None;
        let mut aliases = vec![];
        let mut multiple = false;
        let mut optional = false;
        let mut default = FormFieldDefaultValue::No;
        let mut parse_with = None;

        let attrs = field
            .attrs
            .iter()
            .filter(|a| a.path.is_ident("form"))
            .map(|a| a.parse_meta())
            .map(|v| {
                v.and_then(|m| match m {
                    syn::Meta::List(l) => Ok(l),
                    _ => Err(syn::Error::new_spanned(m, "expected #[form(...)]")),
                })
            });

        for list in attrs {
            let list = list?.nested;
            for meta in list {
                match meta {
                    syn::NestedMeta::Meta(syn::Meta::NameValue(nv))
                        if nv.path.is_ident("rename") =>
                    {
                        if let syn::Lit::Str(s) = nv.lit {
                            name = Some(s.value());
                        } else {
                            return Err(syn::Error::new_spanned(nv.lit, "expected string"));
                        }
                    }
                    syn::NestedMeta::Meta(syn::Meta::NameValue(nv))
                        if nv.path.is_ident("alias") =>
                    {
                        if let syn::Lit::Str(s) = nv.lit {
                            aliases.push(s.value());
                        } else {
                            return Err(syn::Error::new_spanned(nv.lit, "expected string"));
                        }
                    }
                    syn::NestedMeta::Meta(syn::Meta::NameValue(nv))
                        if nv.path.is_ident("multiple") =>
                    {
                        if let syn::Lit::Bool(b) = nv.lit {
                            multiple = b.value;
                        } else {
                            return Err(syn::Error::new_spanned(nv.lit, "expected bool"));
                        }
                    }
                    syn::NestedMeta::Meta(syn::Meta::NameValue(nv))
                        if nv.path.is_ident("default") =>
                    {
                        if let syn::Lit::Bool(b) = nv.lit {
                            if b.value {
                                default = FormFieldDefaultValue::Yes;
                            } else {
                                default = FormFieldDefaultValue::No;
                            }
                        } else if let syn::Lit::Str(s) = nv.lit {
                            default = FormFieldDefaultValue::Custom(syn::parse_str(&s.value())?);
                        } else {
                            return Err(syn::Error::new_spanned(nv.lit, "expected bool or string"));
                        }
                    }
                    syn::NestedMeta::Meta(syn::Meta::NameValue(nv))
                        if nv.path.is_ident("optional") =>
                    {
                        if let syn::Lit::Bool(b) = nv.lit {
                            optional = b.value;
                        } else {
                            return Err(syn::Error::new_spanned(nv.lit, "expected bool"));
                        }
                    }
                    syn::NestedMeta::Meta(syn::Meta::NameValue(nv))
                        if nv.path.is_ident("parse_with") =>
                    {
                        if let syn::Lit::Str(s) = nv.lit {
                            parse_with = Some(syn::parse_str(&s.value())?);
                        } else {
                            return Err(syn::Error::new_spanned(nv.lit, "expected string"));
                        }
                    }
                    syn::NestedMeta::Meta(syn::Meta::Path(p)) if p.is_ident("multiple") => {
                        multiple = true;
                    }
                    syn::NestedMeta::Meta(::syn::Meta::Path(p)) if p.is_ident("optional") => {
                        optional = true;
                    }
                    syn::NestedMeta::Meta(syn::Meta::Path(p)) if p.is_ident("default") => {
                        default = FormFieldDefaultValue::Yes;
                    }

                    v => return Err(syn::Error::new_spanned(v, "expected key-value")),
                }
            }
        }

        let variable_name = field
            .ident
            .clone()
            .map(|n| syn::Ident::new(&format!("__field_{}", n), n.span()))
            .unwrap_or_else(|| {
                syn::Ident::new(&format!("__field_{}", i), proc_macro2::Span::call_site())
            });
        let struct_name = field.ident.clone().unwrap_or_else(|| variable_name.clone());

        if optional && default.has_value() {
            return Err(syn::Error::new_spanned(
                field,
                "cannot have both `default` and `optional`",
            ));
        } else if optional && multiple {
            return Err(syn::Error::new_spanned(
                field,
                "cannot have both `multiple` and `optional`",
            ));
        } else if multiple && default.has_value() {
            return Err(syn::Error::new_spanned(
                field,
                "cannot have both `multiple` and `default`",
            ));
        }

        let ty = match &field.ty {
            syn::Type::Path(p) if optional => match p.path.segments.last().unwrap().arguments {
                syn::PathArguments::AngleBracketed(ref args) => {
                    if let syn::GenericArgument::Type(ref ty) = args.args[0] {
                        ty.clone()
                    } else {
                        Err(syn::Error::new_spanned(field, "expected type in Option"))?
                    }
                }
                _ => Err(syn::Error::new_spanned(field, "expected type in Option"))?,
            },
            _ => field.ty.clone(),
        };

        Ok(dbg!(FormFieldMeta {
            field,
            variable_name,
            struct_name,
            r#type: ty,
            form_key: name
                .or_else(|| {
                    field
                        .ident
                        .as_ref()
                        .map(|ident| rename.apply(&ident.to_string()).into_owned())
                })
                .ok_or_else(|| {
                    syn::Error::new_spanned(field, "expected #[form(rename = \"...\")]")
                })?,
            aliases,
            multiple,
            optional,
            parse_with,
            default,
        }))
    }
}

fn is_named(fields: &syn::Fields) -> bool {
    matches!(fields, syn::Fields::Named(_))
}

fn ident_lit(s: &str, span: proc_macro2::Span) -> syn::Lit {
    syn::Lit::Str(syn::LitStr::new(s, span))
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Rename {
    None,
    LowerCase,
    UpperCase,
    PascalCase,
    CamelCase,
    SnakeCase,
    ScreamingSnakeCase,
    KebabCase,
    ScreamingKebabCase,
}

impl Rename {
    fn from_str(s: &str) -> Result<Rename, String> {
        match s {
            "" => Ok(Rename::None),
            "lowercase" => Ok(Rename::LowerCase),
            "UPPERCASE" => Ok(Rename::UpperCase),
            "PascalCase" => Ok(Rename::PascalCase),
            "camelCase" => Ok(Rename::CamelCase),
            "snake_case" => Ok(Rename::SnakeCase),
            "SCREAMING_SNAKE_CASE" => Ok(Rename::ScreamingSnakeCase),
            "kebab-case" => Ok(Rename::KebabCase),
            "SCREAMING-KEBAB-CASE" => Ok(Rename::ScreamingKebabCase),
            _ => Err(format!("unknown rename rule: {}", s)),
        }
    }

    fn apply<'v>(&self, from: &'v str) -> std::borrow::Cow<'v, str> {
        use heck::*;
        match self {
            Rename::None => from.into(),
            Rename::LowerCase => from.to_lowercase().into(),
            Rename::UpperCase => from.to_uppercase().into(),
            Rename::PascalCase => from.to_pascal_case().into(),
            Rename::CamelCase => from.to_lower_camel_case().into(),
            Rename::SnakeCase => from.to_snake_case().into(),
            Rename::ScreamingSnakeCase => from.to_shouty_snake_case().into(),
            Rename::KebabCase => from.to_kebab_case().into(),
            Rename::ScreamingKebabCase => from.to_shouty_kebab_case().into(),
        }
    }
}
