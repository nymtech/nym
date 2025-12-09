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
//!     user_agent = "Nym/1.0"
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
//!         "Authorization" => "auth_token"
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
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    Expr, Ident, LitInt, Result, Token, braced,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    token,
};

// ------------------ core crate path resolution ------------------

fn core_path() -> TokenStream2 {
    use proc_macro_crate::{FoundCrate, crate_name};

    match crate_name("nym-http-api-client") {
        Ok(FoundCrate::Itself) => quote!(crate),
        Ok(FoundCrate::Name(name)) => {
            let ident = Ident::new(&name, proc_macro2::Span::call_site());
            quote!( ::#ident )
        }
        Err(_) => quote!(::nym_http_api_client),
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
    },
    Call {
        key: Ident,
        args: Punctuated<Expr, Token![,]>,
        _p: token::Paren,
    },
    DefaultHeaders {
        _key: Ident,
        map: HeaderMapInit,
    },
    Flag {
        key: Ident,
    },
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

        if input.peek(token::Brace) && key == quote::format_ident!("default_headers") {
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
            let _ = input.parse::<Token![=]>()?; // =
            let lit: LitInt = input.parse()?;
            priority = lit.base10_parse()?;
            let _ = input.parse::<Token![;]>()?; // ;
        }

        let items = input.parse::<Items>()?;
        Ok(Self { priority, items })
    }
}

fn describe_items(items: &Items) -> String {
    use std::fmt::Write;

    let mut buf = String::new();

    for (idx, item) in items.0.iter().enumerate() {
        if idx > 0 {
            buf.push_str(", ");
        }

        match item {
            Item::Assign { key, value, .. } => {
                let k = quote!(#key).to_string();
                let v = quote!(#value).to_string();
                let _ = write!(buf, "{}={}", k, v);
            }
            Item::Call { key, args, .. } => {
                let k = quote!(#key).to_string();
                let args_str = args
                    .iter()
                    .map(|a| quote!(#a).to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                let _ = write!(buf, "{}({})", k, args_str);
            }
            Item::Flag { key } => {
                let k = quote!(#key).to_string();
                let _ = write!(buf, "{}()", k);
            }
            Item::DefaultHeaders { .. } => {
                buf.push_str("default_headers{...}");
            }
        }
    }

    buf
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
/// use nym_http_api_client_macro::client_cfg;
///
/// let config = client_cfg!(
///     timeout = std::time::Duration::from_secs(30),
///     gzip = true
/// );
/// let client = config(reqwest::ClientBuilder::new()).build().unwrap();
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
/// use nym_http_api_client_macro::client_defaults;
///
/// client_defaults!(
///     priority = -50;  // Run early in the configuration chain
///     connect_timeout = std::time::Duration::from_secs(10),
///     pool_max_idle_per_host = 32,
///     default_headers {
///         "User-Agent" => "Nym/1.0",
///         "Accept" => "application/json"
///     }
/// );
/// ```
#[proc_macro]
pub fn client_defaults(input: TokenStream) -> TokenStream {
    let MaybePrioritized { priority, items } = parse_macro_input!(input as MaybePrioritized);
    let core = core_path();

    // Deterministic debug description string (used only when debug feature is enabled).
    let description = describe_items(&items);

    // Turn the DSL into statements that mutate `b`.
    let body = to_stmts(items, &core);

    // Optional compile-time diagnostics for the macro author (does not affect output).
    if std::env::var("DEBUG_HTTP_INVENTORY").is_ok() {
        eprintln!(
            "cargo:warning=[HTTP-INVENTORY] Registering config with priority={} from {}: {}",
            priority,
            std::env::var("CARGO_PKG_NAME").unwrap_or_else(|_| "unknown".to_string()),
            description,
        );
    }

    // Debug logging injected into the generated closure, gated by the
    // *macro crate's* `debug-inventory` feature (checked at expansion time).
    let debug_block = if cfg!(feature = "debug-inventory") {
        quote! {
            eprintln!(
                "[HTTP-INVENTORY] Applying: {} (priority={})",
                #description,
                #priority
            );
        }
    } else {
        quote! {}
    };

    // `apply` is a capture-free closure; it will coerce to a fn pointer
    // if `ConfigRecord::apply` is typed as `fn(ReqwestClientBuilder) -> ReqwestClientBuilder`.
    let out = quote! {
        #core::inventory::submit! {
            #core::registry::ConfigRecord {
                priority: #priority,
                apply: |mut b: #core::ReqwestClientBuilder| {
                    #debug_block
                    #body
                    b
                },
            }
        }
    };

    out.into()
}
