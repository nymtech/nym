//! Proc-macros for configuring HTTP clients globally via the `inventory` crate.
//!
//! This crate provides macros that allow any crate in the workspace to contribute
//! configuration modifications to `reqwest::ClientBuilder` instances through a
//! compile-time registry pattern.
//!
//! # Overview
//!
//! The macros work by:
//! 1. Collecting configuration functions from across all crates at compile time
//! 2. Sorting them by priority (lower numbers run first)
//! 3. Applying them sequentially to build HTTP clients with consistent settings
//!
//! # Examples
//!
//! ## Basic Usage with `client_defaults!`
//!
//! ```ignore
//! use nym_http_api_client_macro::client_defaults;
//!
//! // Register default configurations with priority
//! client_defaults!(
//!     priority = 10;  // Optional, defaults to 0
//!     timeout = std::time::Duration::from_secs(30),
//!     gzip = true,
//!     user_agent = "MyApp/1.0"
//! );
//! ```
//!
//! ## Using `client_cfg!` for one-off configurations
//!
//! ```ignore
//! use nym_http_api_client_macro::client_cfg;
//!
//! let configure = client_cfg!(
//!     timeout = std::time::Duration::from_secs(60),
//!     default_headers {
//!         "X-Custom-Header" => "value",
//!         "Authorization" => auth_token
//!     }
//! );
//!
//! let builder = reqwest::ClientBuilder::new();
//! let configured = configure(builder);
//! ```
//!
//! # DSL Reference
//!
//! The macro DSL supports several patterns:
//! - `key = value` - Calls `builder.key(value)`
//! - `key(arg1, arg2)` - Calls `builder.key(arg1, arg2)`
//! - `flag` - Calls `builder.flag()` with no arguments
//! - `default_headers { "name" => "value", ... }` - Sets default headers
//!
//! # Priority System
//!
//! Configurations are applied in priority order (lower numbers first):
//! - Negative priorities: Early configuration (e.g., -100 for base settings)
//! - Zero (default): Standard configuration
//! - Positive priorities: Late configuration (e.g., 100 for overrides)

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use proc_macro_crate::{crate_name, FoundCrate};
use quote::{format_ident, quote};
use syn::{
    braced,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    token, Expr, Ident, LitInt, Result, Token,
};

// ------------------ core crate path resolution ------------------

fn core_path() -> TokenStream2 {
    match crate_name("nym-http-api-client") {
        Ok(FoundCrate::Itself) => quote!(crate),
        Ok(FoundCrate::Name(name)) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!( ::#ident )
        }
        Err(_) => {
            // Fallback if the crate is not found by name (unlikely if deps set up correctly)
            quote!(::nym_http_api_client)
        }
    }
}

// ------------------ DSL parsing ------------------

struct Items(Punctuated<Item, Token![,]>);
impl Parse for Items {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        Ok(Self(Punctuated::parse_terminated(input)?))
    }
}

enum Item {
    Assign {
        key: Ident,
        _eq: Token![=],
        value: Expr,
    }, // foo = EXPR
    Call {
        key: Ident,
        args: Punctuated<Expr, Token![,]>,
        _p: token::Paren,
    }, // foo(a,b)
    DefaultHeaders {
        _key: Ident,
        map: HeaderMapInit,
    }, // default_headers { ... }
    Flag {
        key: Ident,
    }, // foo
}

impl Parse for Item {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let key: Ident = input.parse()?;

        if input.peek(Token![=]) {
            let _eq: Token![=] = input.parse()?;
            let value: Expr = input.parse()?;
            return Ok(Self::Assign { key, _eq, value });
        }
        if input.peek(token::Paren) {
            let content;
            let _p = syn::parenthesized!(content in input);
            let args = Punctuated::<Expr, Token![,]>::parse_terminated(&content)?;
            return Ok(Self::Call { key, args, _p });
        }
        if input.peek(token::Brace) && key == format_ident!("default_headers") {
            let map = input.parse::<HeaderMapInit>()?;
            return Ok(Self::DefaultHeaders { _key: key, map });
        }
        Ok(Self::Flag { key })
    }
}

struct HeaderPair {
    k: Expr,
    _arrow: Token![=>],
    v: Expr,
}
impl Parse for HeaderPair {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        Ok(Self {
            k: input.parse()?,
            _arrow: input.parse()?,
            v: input.parse()?,
        })
    }
}

struct HeaderMapInit {
    _brace: token::Brace,
    pairs: Punctuated<HeaderPair, Token![,]>,
}
impl Parse for HeaderMapInit {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let content;
        let _brace = braced!(content in input);
        let pairs = Punctuated::<HeaderPair, Token![,]>::parse_terminated(&content)?;
        Ok(Self { _brace, pairs })
    }
}

// Generate statements that mutate a builder named `b` using the resolved core path.
fn to_stmts(items: Items, core: &TokenStream2) -> TokenStream2 {
    let mut stmts = Vec::new();
    for it in items.0 {
        match it {
            Item::Assign { key, value, .. } => {
                let m = key;
                stmts.push(quote! { b = b.#m(#value); });
            }
            Item::Call { key, args, .. } => {
                let m = key;
                let args = args.iter();
                stmts.push(quote! { b = b.#m( #( #args ),* ); });
            }
            Item::DefaultHeaders { map, .. } => {
                let (ks, vs): (Vec<_>, Vec<_>) = map.pairs.into_iter().map(|p| (p.k, p.v)).unzip();
                stmts.push(quote! {
                    let mut __cm = #core::reqwest::header::HeaderMap::new();
                    #(
                        {
                            use #core::reqwest::header::{HeaderName, HeaderValue};
                            let __k = HeaderName::try_from(#ks)
                                .unwrap_or_else(|e| panic!("Invalid header name: {}", e));
                            let __v = HeaderValue::try_from(#vs)
                                .unwrap_or_else(|e| panic!("Invalid header value: {}", e));
                            __cm.insert(__k, __v);
                        }
                    )*
                    b = b.default_headers(__cm);
                });
            }
            Item::Flag { key } => {
                let m = key;
                stmts.push(quote! { b = b.#m(); });
            }
        }
    }
    quote! { #(#stmts)* }
}

// ------------------ client_cfg! ------------------

/// Creates a closure that configures a `ReqwestClientBuilder`.
///
/// This macro generates a closure that can be used to configure a single
/// `reqwest::ClientBuilder` instance without affecting global defaults.
///
/// # Example
///
/// ```ignore
/// let config = client_cfg!(
///     timeout = Duration::from_secs(30),
///     gzip = true
/// );
/// let client = config(reqwest::ClientBuilder::new()).build()?;
/// ```
#[proc_macro]
pub fn client_cfg(input: TokenStream) -> TokenStream {
    let items = parse_macro_input!(input as Items);
    let core = core_path();
    let body = to_stmts(items, &core);
    let out = quote! {
        |mut b: #core::ReqwestClientBuilder| { #body b }
    };
    out.into()
}

// ------------------ client_defaults! with optional priority header ------------------

struct MaybePrioritized {
    priority: i32,
    items: Items,
}
impl Parse for MaybePrioritized {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        // Optional header: `priority = <int> ;`
        let fork = input.fork();
        let mut priority = 0i32;
        if fork.peek(Ident) && fork.parse::<Ident>()? == "priority" && fork.peek(Token![=]) {
            // commit
            let _ = input.parse::<Ident>()?; // priority
            let _ = input.parse::<Token![=]>()?;
            let lit: LitInt = input.parse()?;
            priority = lit.base10_parse()?;
            let _ = input.parse::<Token![;]>()?;
        }
        let items = input.parse::<Items>()?;
        Ok(Self { priority, items })
    }
}

/// Registers global default configurations for HTTP clients.
///
/// This macro submits a configuration record to the global registry that will
/// be applied to all HTTP clients created with `default_builder()`.
///
/// # Parameters
///
/// - `priority` (optional): Integer priority for ordering (lower runs first, default: 0)
/// - Configuration items: Any valid `reqwest::ClientBuilder` method calls
///
/// # Example
///
/// ```ignore
/// client_defaults!(
///     priority = -50;  // Run early in the configuration chain
///     connect_timeout = Duration::from_secs(10),
///     pool_max_idle_per_host = 32,
///     default_headers {
///         "User-Agent" => "MyApp/1.0",
///         "Accept" => "application/json"
///     }
/// );
/// ```
#[proc_macro]
pub fn client_defaults(input: TokenStream) -> TokenStream {
    let MaybePrioritized { priority, items } = parse_macro_input!(input as MaybePrioritized);
    let core = core_path();
    let body = to_stmts(items, &core);

    let out = quote! {
        #[allow(non_snake_case)]
        mod __client_defaults {
            use super::*;
            #[allow(unused)]
            pub fn __cfg(
                mut b: #core::ReqwestClientBuilder
            ) -> #core::ReqwestClientBuilder {
                #body
                b
            }

            #core::inventory::submit! {
                #core::registry::ConfigRecord {
                    priority: #priority,
                    apply: __cfg,
                }
            }
        }
    };
    out.into()
}
