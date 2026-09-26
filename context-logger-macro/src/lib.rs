//! Attribute macros for `context-logger`.

#![allow(missing_docs)]

use manyhow::manyhow;
use proc_macro2::TokenStream as TokenStream2;

mod args;
mod expand;

#[manyhow]
#[proc_macro_attribute]
pub fn log_scope(args: args::Args, item: syn::ItemFn) -> manyhow::Result<TokenStream2> {
    expand::expand(args, item).map_err(Into::into)
}
