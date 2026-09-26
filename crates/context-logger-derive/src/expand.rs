use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, ItemFn, Result, spanned::Spanned};

use crate::args::{ContextFields, Field, Mode, Section};

pub fn expand(args: &ContextFields, mut function: ItemFn) -> Result<TokenStream> {
    reject_const_function(&function)?;

    let crate_path = crate_path()?;
    let context = context_expression(args, &crate_path)?;
    let body = &function.block;

    function.block = if function.sig.asyncness.is_some() {
        syn::parse_quote!({
            let __log_scope_context = #context;
            #crate_path::FutureExt::in_log_context(
                async #body,
                __log_scope_context,
            )
            .await
        })
    } else {
        syn::parse_quote!({
            let __log_scope_context = #context;
            #crate_path::LogContextExt::in_scope(
                __log_scope_context,
                || #body,
            )
        })
    };

    Ok(quote!(#function))
}

fn reject_const_function(function: &ItemFn) -> Result<()> {
    if let Some(constness) = function.sig.constness {
        return Err(Error::new(
            constness.span(),
            "#[log_scope] cannot be applied to const functions",
        ));
    }

    Ok(())
}

fn context_expression(args: &ContextFields, crate_path: &TokenStream) -> Result<TokenStream> {
    let mut expression = quote!(#crate_path::LogContext::new());

    for section in &args.sections {
        expression = append_section(expression, section, crate_path)?;
    }

    Ok(expression)
}

fn append_section(
    context: TokenStream,
    section: &Section,
    crate_path: &TokenStream,
) -> Result<TokenStream> {
    let (fields, method) = match section {
        Section::LocalFields(fields) => (fields, quote!(with_local_field)),
        Section::InheritedFields(fields) => (fields, quote!(with_inherited_field)),
    };

    fields.iter().try_fold(context, |context, field| {
        let key = field.key.name();
        let value = field_value(field, crate_path)?;
        Ok(quote!(#context.#method(#key, #value)))
    })
}

fn field_value(field: &Field, crate_path: &TokenStream) -> Result<TokenStream> {
    let value = &field.value;

    match field.mode {
        Mode::Default => Ok(quote!(#value)),
        Mode::Debug => Ok(quote!(#crate_path::LogValue::debug(#value))),
        Mode::Display => Ok(quote!(#crate_path::LogValue::display(#value))),
        Mode::Error => Ok(quote!(#crate_path::LogValue::error(#value))),
        Mode::Serde => Ok(quote!(#crate_path::LogValue::serde(#value))),
        Mode::Sval => Err(Error::new(
            field.key.span(),
            "`sval` capture is not supported by context-logger",
        )),
    }
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
