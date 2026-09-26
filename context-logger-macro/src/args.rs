use std::collections::HashSet;

use strum::EnumDiscriminants;
use syn::{
    Expr, Ident, LitStr, Result, Token, parenthesized,
    parse::{Parse, ParseStream},
};

#[derive(Debug)]
pub struct ContextFields {
    pub sections: Vec<Section>,
}

#[derive(Debug, EnumDiscriminants)]
#[strum_discriminants(name(SectionKind))]
pub enum Section {
    LocalFields(Vec<Field>),
    InheritedFields(Vec<Field>),
}

#[derive(Debug)]
pub struct Field {
    pub key: Key,
    pub mode: Mode,
    pub value: Expr,
}

#[derive(Debug)]
pub enum Key {
    Ident(Ident),
    String(LitStr),
}

#[derive(Debug, Clone, Copy)]
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
        let mut local_fields_seen = false;
        let mut inherited_fields_seen = false;
        while !input.is_empty() {
            let name: Ident = input.parse()?;
            let section_kind = SectionKind::try_from(&name)?;
            match section_kind {
                SectionKind::LocalFields if local_fields_seen => {
                    return Err(syn::Error::new(name.span(), "duplicate log_scope section"));
                }
                SectionKind::InheritedFields if inherited_fields_seen => {
                    return Err(syn::Error::new(name.span(), "duplicate log_scope section"));
                }
                SectionKind::LocalFields => local_fields_seen = true,
                SectionKind::InheritedFields => inherited_fields_seen = true,
            }
            let content;
            parenthesized!(content in input);
            let fields = parse_fields(&content, &mut seen_keys)?;
            sections.push(match section_kind {
                SectionKind::LocalFields => Section::LocalFields(fields),
                SectionKind::InheritedFields => Section::InheritedFields(fields),
            });
            let _ = input.parse::<Token![,]>();
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

fn parse_fields(input: ParseStream<'_>, seen_keys: &mut HashSet<String>) -> Result<Vec<Field>> {
    let mut fields = Vec::new();
    while !input.is_empty() {
        let key = if input.peek(LitStr) {
            Key::String(input.parse()?)
        } else {
            Key::Ident(input.parse()?)
        };
        let mode = if input.peek(Token![:]) {
            input.parse::<Token![:]>()?;
            parse_mode(input)?
        } else {
            Mode::Default
        };
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
        let key_text = match &key {
            Key::Ident(i) => i.to_string(),
            Key::String(s) => s.value(),
        };
        if !seen_keys.insert(key_text) {
            return Err(syn::Error::new_spanned(
                key_token(&key),
                "duplicate log_scope field",
            ));
        }
        fields.push(Field { key, mode, value });
        if input.is_empty() {
            break;
        }
        input.parse::<Token![,]>()?;
    }
    Ok(fields)
}

fn parse_mode(input: ParseStream<'_>) -> Result<Mode> {
    if input.peek(Token![?]) {
        input.parse::<Token![?]>()?;
        return Ok(Mode::Debug);
    }
    if input.peek(Token![%]) {
        input.parse::<Token![%]>()?;
        return Ok(Mode::Display);
    }
    let ident: Ident = input.parse()?;
    match ident.to_string().as_str() {
        "debug" => Ok(Mode::Debug),
        "display" => Ok(Mode::Display),
        "err" => Ok(Mode::Error),
        "serde" => Ok(Mode::Serde),
        "sval" => Ok(Mode::Sval),
        _ => Err(syn::Error::new(
            ident.span(),
            "unknown log_scope capture modifier",
        )),
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
    fn rejects_string_shorthand() {
        assert!(parse_str::<ContextFields>(r#"local_fields("field")"#).is_err());
    }
}
