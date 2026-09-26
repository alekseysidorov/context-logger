use std::collections::HashSet;

use strum::EnumDiscriminants;
use syn::{
    Expr, Ident, LitStr, Result, Token, parenthesized,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

#[derive(Clone, Debug)]
pub struct ContextFields {
    pub sections: Vec<Section>,
}

#[derive(Clone, Debug, EnumDiscriminants)]
#[strum_discriminants(name(SectionKind), derive(Hash))]
pub enum Section {
    LocalFields(Vec<Field>),
    InheritedFields(Vec<Field>),
}

#[derive(Clone, Debug)]
pub struct Field {
    pub key: Key,
    pub mode: Mode,
    pub value: Expr,
}

#[derive(Clone, Debug)]
pub enum Key {
    Ident(Ident),
    String(LitStr),
}

impl Key {
    pub(crate) fn name(&self) -> String {
        match self {
            Self::Ident(ident) => ident.to_string(),
            Self::String(string) => string.value(),
        }
    }
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

impl Parse for ContextFields {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut sections = Vec::new();
        let mut seen_keys = HashSet::new();
        let mut seen_sections = HashSet::new();
        while !input.is_empty() {
            let name: Ident = input.parse()?;
            let section_kind = SectionKind::try_from(&name)?;

            if !seen_sections.insert(section_kind) {
                return Err(syn::Error::new(
                    name.span(),
                    format!("duplicate `{name}` log_scope section"),
                ));
            }

            let content;
            parenthesized!(content in input);
            let fields = parse_fields(&content, &mut seen_keys)?;
            sections.push(match section_kind {
                SectionKind::LocalFields => Section::LocalFields(fields),
                SectionKind::InheritedFields => Section::InheritedFields(fields),
            });
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

impl Parse for Key {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        if input.peek(LitStr) {
            Ok(Self::String(input.parse()?))
        } else {
            Ok(Self::Ident(input.parse()?))
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
                Key::Ident(ident) => syn::parse_quote!(#ident),
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

fn parse_fields(input: ParseStream<'_>, seen_keys: &mut HashSet<String>) -> Result<Vec<Field>> {
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
    use syn::parse_str;

    use super::*;

    impl Section {
        fn all_fields(&self) -> &[Field] {
            match self {
                Self::LocalFields(fields) | Self::InheritedFields(fields) => fields,
            }
        }
    }

    #[test]
    fn parses_sections_and_capture_modes() {
        let args: ContextFields = parse_str(
            r#"
                inherited_fields(request_id, user:? = user, "http.method":% = request.method),
                local_fields(operation = "load", payload:serde)
            "#,
        )
        .unwrap();

        assert_eq!(args.sections.len(), 2);
        assert!(matches!(args.sections[0], Section::InheritedFields(_)));
        assert!(matches!(args.sections[1], Section::LocalFields(_)));
        assert!(matches!(
            args.sections[0].all_fields()[0].mode,
            Mode::Default
        ));
        assert!(matches!(args.sections[0].all_fields()[1].mode, Mode::Debug));
        assert!(matches!(
            args.sections[0].all_fields()[2].mode,
            Mode::Display
        ));
        assert!(matches!(args.sections[1].all_fields()[1].mode, Mode::Serde));
    }

    #[test]
    fn parses_all_modifier_spellings() {
        let args: ContextFields = parse_str(
            "local_fields(a:? = a, b:debug = b, c:% = c, d:display = d, e:err = e, f:serde = f, g:sval = g)",
        )
        .unwrap();

        let modes = args.sections[0]
            .all_fields()
            .iter()
            .map(|field| field.mode)
            .collect::<Vec<_>>();
        assert!(matches!(
            modes.as_slice(),
            [
                Mode::Debug,
                Mode::Debug,
                Mode::Display,
                Mode::Display,
                Mode::Error,
                Mode::Serde,
                Mode::Sval
            ]
        ));
    }

    #[test]
    fn rejects_duplicate_sections_and_keys() {
        assert!(parse_str::<ContextFields>("local_fields(a = 1), local_fields(b = 2)").is_err());
        assert!(parse_str::<ContextFields>("local_fields(a = 1, a = 2)").is_err());
        assert!(parse_str::<ContextFields>("other_fields(a = 1)").is_err());
    }

    #[test]
    fn rejects_missing_section_separator() {
        assert!(parse_str::<ContextFields>("local_fields(a = 1) inherited_fields(b = 2)").is_err());
    }

    #[test]
    fn rejects_string_shorthand() {
        assert!(parse_str::<ContextFields>(r#"local_fields("field")"#).is_err());
    }
}
