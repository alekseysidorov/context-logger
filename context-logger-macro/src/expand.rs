use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Error, ItemFn, Result, spanned::Spanned};

use crate::args::{Args, Field, Key, Mode};

pub fn expand(args: Args, mut function: ItemFn) -> Result<TokenStream> {
    if function.sig.constness.is_some() {
        return Err(Error::new(
            function.sig.constness.span(),
            "#[log_scope] cannot be applied to const functions",
        ));
    }
    if function.sig.asyncness.is_some() {
        expand_async(args, &mut function)
    } else {
        expand_sync(args, &mut function)
    }
}

fn expand_sync(args: Args, function: &mut ItemFn) -> Result<TokenStream> {
    let context = context_expression(&args)?;
    let body = &function.block;
    let crate_path = crate_path()?;
    function.block = syn::parse_quote!({
        let __log_scope_context = #context;
        #crate_path::LogContextExt::in_scope(__log_scope_context, || #body)
    });
    Ok(quote!(#function))
}

fn expand_async(args: Args, function: &mut ItemFn) -> Result<TokenStream> {
    let context = context_expression(&args)?;
    let body = &function.block;
    let crate_path = crate_path()?;
    function.block = syn::parse_quote!({
        let __log_scope_context = #context;
        #crate_path::FutureExt::in_log_context(async #body, __log_scope_context).await
    });
    Ok(quote!(#function))
}

fn context_expression(args: &Args) -> Result<TokenStream> {
    let crate_path = crate_path()?;
    let mut expression = quote!(#crate_path::LogContext::new());
    let mut seen = std::collections::HashSet::new();
    for section in &args.sections {
        for field in &section.fields {
            let key = field.key_text();
            if !seen.insert(key) {
                return Err(Error::new(field_span(field), "duplicate log_scope field"));
            }
            let key = match &field.key {
                Key::Ident(key) => quote!(stringify!(#key)),
                Key::String(key) => quote!(#key),
            };
            let value = match field.mode {
                Mode::Default => {
                    let value = &field.value;
                    quote!(#value)
                }
                Mode::Debug => wrap_value(&crate_path, "debug", &field.value),
                Mode::Display => wrap_value(&crate_path, "display", &field.value),
                Mode::Error => wrap_value(&crate_path, "error", &field.value),
                Mode::Serde => wrap_value(&crate_path, "serde", &field.value),
                Mode::Sval => {
                    return Err(Error::new(
                        field_span(field),
                        "`sval` capture is not supported by context-logger",
                    ));
                }
            };
            let method = if section.inherited {
                format_ident!("with_inherited_field")
            } else {
                format_ident!("with_local_field")
            };
            expression = quote!(#expression.#method(#key, #value));
        }
    }
    Ok(expression)
}

fn wrap_value(crate_path: &TokenStream, method: &str, value: &syn::Expr) -> TokenStream {
    let method = syn::Ident::new(method, value.span());
    quote!(#crate_path::LogValue::#method(#value))
}

fn crate_path() -> Result<TokenStream> {
    match crate_name("context-logger") {
        Ok(FoundCrate::Itself) => Ok(quote!(::context_logger)),
        Ok(FoundCrate::Name(name)) => {
            let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
            Ok(quote!(::#ident))
        }
        Err(error) => Err(Error::new(
            proc_macro2::Span::call_site(),
            format!("unable to resolve context-logger crate: {error}"),
        )),
    }
}

fn field_span(field: &Field) -> proc_macro2::Span {
    match &field.key {
        Key::Ident(key) => key.span(),
        Key::String(key) => key.span(),
    }
}

trait FieldExt {
    fn key_text(&self) -> String;
}
impl FieldExt for Field {
    fn key_text(&self) -> String {
        match &self.key {
            Key::Ident(key) => key.to_string(),
            Key::String(key) => key.value(),
        }
    }
}
