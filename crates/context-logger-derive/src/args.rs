use std::collections::HashSet;

use strum::EnumDiscriminants;
use syn::{
    Expr, Ident, LitStr, Result, Token,
    ext::IdentExt,
    parenthesized,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextFields {
    pub sections: Vec<Section>,
}

#[derive(Clone, Debug, EnumDiscriminants, PartialEq, Eq)]
#[strum_discriminants(name(SectionKind), derive(Hash))]
pub enum Section {
    LocalFields(Vec<Field>),
    InheritedFields(Vec<Field>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    pub key: Key,
    pub mode: Mode,
    pub value: Expr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Key {
    Ident(Ident),
    String(LitStr),
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Mode {
    Default,
    Debug,
    Display,
    Error,
    Serde,
    Sval,
}

impl ContextFields {
    fn parse_fields(input: ParseStream<'_>) -> Result<Vec<Field>> {
        let mut seen_keys = HashSet::new();

        Punctuated::<Field, Token![,]>::parse_terminated(input)?
            .into_iter()
            .map(|field| {
                if !seen_keys.insert(field.key.name()) {
                    return Err(syn::Error::new_spanned(
                        key_token(&field.key),
                        "duplicate log_scope field",
                    ));
                }

                Ok(field)
            })
            .collect()
    }
}

impl Parse for ContextFields {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut seen_sections = HashSet::new();

        let mut sections = Vec::new();
        while !input.is_empty() {
            let name: Ident = input.parse()?;
            let section_kind = SectionKind::try_from(&name)?;

            if !seen_sections.insert(section_kind) {
                return Err(syn::Error::new(
                    name.span(),
                    format!("duplicate `{name}` log_scope section"),
                ));
            }

            let fields = {
                let content;
                parenthesized!(content in input);
                Self::parse_fields(&content)?
            };
            sections.push(Section::new(section_kind, fields));

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(Self { sections })
    }
}

impl TryFrom<&Ident> for SectionKind {
    type Error = syn::Error;

    fn try_from(ident: &Ident) -> Result<Self> {
        match ident.to_string().as_str() {
            "local_fields" => Ok(Self::LocalFields),
            "inherited_fields" => Ok(Self::InheritedFields),
            _ => Err(syn::Error::new(
                ident.span(),
                format!("unknown log_scope section: {ident}"),
            )),
        }
    }
}

impl Section {
    const fn new(kind: SectionKind, fields: Vec<Field>) -> Self {
        match kind {
            SectionKind::LocalFields => Self::LocalFields(fields),
            SectionKind::InheritedFields => Self::InheritedFields(fields),
        }
    }
}

impl Key {
    pub(crate) fn name(&self) -> String {
        match self {
            Self::Ident(ident) => ident.to_string(),
            Self::String(string) => string.value(),
        }
    }

    pub(crate) fn span(&self) -> proc_macro2::Span {
        match self {
            Self::Ident(ident) => ident.span(),
            Self::String(string) => string.span(),
        }
    }
}

impl Parse for Key {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        if input.peek(LitStr) {
            Ok(Self::String(input.parse()?))
        } else {
            Ok(Self::Ident(Ident::parse_any(input)?))
        }
    }
}

impl Parse for Field {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        // A field starts with either an identifier key or a string-literal key.
        let key: Key = input.parse()?;
        // Capture modifiers use the `:modifier` syntax from `log`'s KV macros.
        let mode = if input.peek(Token![:]) {
            input.parse::<Token![:]>()?;
            input.parse()?
        } else {
            Mode::Default
        };
        // An explicit value follows `=`; an identifier key may use shorthand.
        let value = if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            input.parse()?
        } else {
            match &key {
                Key::Ident(ident) => syn::parse2(quote::quote!(#ident)).map_err(|_| {
                    syn::Error::new(ident.span(), "shorthand requires an identifier key")
                })?,
                Key::String(lit) => {
                    return Err(syn::Error::new(
                        lit.span(),
                        "shorthand requires an identifier key",
                    ));
                }
            }
        };

        Ok(Self { key, mode, value })
    }
}

impl Parse for Mode {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        // `?` and `%` are punctuation tokens rather than identifiers, so the
        // short forms must be parsed before the named forms below.
        if input.peek(Token![?]) {
            input.parse::<Token![?]>()?;
            return Ok(Self::Debug);
        }
        if input.peek(Token![%]) {
            input.parse::<Token![%]>()?;
            return Ok(Self::Display);
        }

        let ident: Ident = input.parse()?;
        match ident.to_string().as_str() {
            "debug" => Ok(Self::Debug),
            "display" => Ok(Self::Display),
            "err" => Ok(Self::Error),
            "serde" => Ok(Self::Serde),
            "sval" => Ok(Self::Sval),
            _ => Err(syn::Error::new(
                ident.span(),
                "unknown log_scope capture modifier",
            )),
        }
    }
}

fn key_token(key: &Key) -> proc_macro2::TokenStream {
    match key {
        Key::Ident(i) => quote::quote!(#i),
        Key::String(s) => quote::quote!(#s),
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use syn::{parse_quote, parse_str};

    use super::*;

    // Happy-path coverage for sections, keys, shorthand, and capture modes.
    #[test]
    fn parses_sections_and_capture_modes() {
        let actual: ContextFields = parse_str(
            r#"
                inherited_fields(request_id, user:? = user, "http.method":% = request.method),
                local_fields(operation = "load", payload:serde)
            "#,
        )
        .unwrap();

        let expected = ContextFields {
            sections: vec![
                Section::InheritedFields(vec![
                    Field {
                        key: Key::Ident(parse_quote!(request_id)),
                        mode: Mode::Default,
                        value: parse_quote!(request_id),
                    },
                    Field {
                        key: Key::Ident(parse_quote!(user)),
                        mode: Mode::Debug,
                        value: parse_quote!(user),
                    },
                    Field {
                        key: Key::String(parse_quote!("http.method")),
                        mode: Mode::Display,
                        value: parse_quote!(request.method),
                    },
                ]),
                Section::LocalFields(vec![
                    Field {
                        key: Key::Ident(parse_quote!(operation)),
                        mode: Mode::Default,
                        value: parse_quote!("load"),
                    },
                    Field {
                        key: Key::Ident(parse_quote!(payload)),
                        mode: Mode::Serde,
                        value: parse_quote!(payload),
                    },
                ]),
            ],
        };

        assert_eq!(actual, expected);
    }

    // Ensure punctuation and named spellings map to the same capture modes.
    #[test]
    fn parses_all_modifier_spellings() {
        let actual: ContextFields = parse_str(
            "local_fields(a:? = a, b:debug = b, c:% = c, d:display = d, e:err = e, f:serde = f, g:sval = g)",
        )
        .unwrap();

        let expected = ContextFields {
            sections: vec![Section::LocalFields(vec![
                parse_quote!(a:? = a),
                parse_quote!(b:debug = b),
                parse_quote!(c:% = c),
                parse_quote!(d:display = d),
                parse_quote!(e:err = e),
                parse_quote!(f:serde = f),
                parse_quote!(g:sval = g),
            ])],
        };

        assert_eq!(actual, expected);
    }

    // Validation coverage for duplicate sections, duplicate fields, and unknown parser inputs.
    #[test]
    fn rejects_duplicate_sections_and_fields() {
        assert!(parse_str::<ContextFields>("local_fields(a = 1), local_fields(b = 2)").is_err());
        assert!(parse_str::<ContextFields>("local_fields(a = 1, a = 2)").is_err());
        assert!(parse_str::<ContextFields>("inherited_fields(a = 1, a = 2)").is_err());
        assert!(parse_str::<ContextFields>("other_fields(a = 1)").is_err());
    }

    // Local and inherited fields are separate declaration namespaces.
    #[test]
    fn accepts_same_field_in_local_and_inherited_sections() {
        let fields: ContextFields =
            parse_str("inherited_fields(value = inherited), local_fields(value = local)").unwrap();

        assert_eq!(fields.sections.len(), 2);
    }

    // The section list must use commas as separators.
    #[test]
    fn rejects_missing_section_separator() {
        assert!(parse_str::<ContextFields>("local_fields(a = 1) inherited_fields(b = 2)").is_err());
    }

    // String keys require an explicit value and cannot use shorthand.
    #[test]
    fn rejects_string_shorthand() {
        assert!(parse_str::<ContextFields>(r#"local_fields("field")"#).is_err());
    }
}
