//! Proc-macros for configuring reqwest clients via a tiny DSL and a link-time registry.
//!
//! - `client_cfg!(...)` -> returns `impl FnOnce(reqwest::ClientBuilder) -> reqwest::ClientBuilder`
//! - `#[client_defaults(...)]` on a module that defines `pub fn __cfg(...) -> ...`
//! - registers that function into the `inventory` registry owned by `common_http`.

use proc_macro::TokenStream;
use syn::parse::Parser;
use quote::{format_ident, quote};
use syn::{
    braced,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    token, Expr, Ident, ItemMod, LitInt, LitStr, Result, Token,
};

// ======================
// Builder-DSL parsing
// ======================

struct Items(Punctuated<Item, Token![,]>);
impl Parse for Items {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        Ok(Self(Punctuated::parse_terminated(input)?))
    }
}

enum Item {
    /// `foo = EXPR` (sugar for `foo(EXPR)`)
    Assign {
        key: Ident,
        _eq: Token![=],
        value: Expr,
    },
    /// `foo(arg1, arg2, ...)`
    Call {
        key: Ident,
        args: Punctuated<Expr, Token![,]>,
        _p: token::Paren,
    },
    /// `default_headers { "K" => "V", ... }`
    DefaultHeaders {
        _key: Ident,
        map: HeaderMapInit,
    },
    /// `foo` (zero-arg method)
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

        // Recognize `default_headers { ... }`
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

/// Turn `Items` into statements that mutate a variable named `b: ::common_http::reqwest::ClientBuilder`.
fn to_stmts(items: Items) -> proc_macro2::TokenStream {
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
                    let mut __cm = ::common_http::reqwest::header::HeaderMap::new();
                    #(
                        {
                            use ::common_http::reqwest::header::{HeaderName, HeaderValue};
                            let __k = HeaderName::try_from(#ks).expect("invalid header name");
                            let __v = HeaderValue::try_from(#vs).expect("invalid header value");
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

// ======================
// client_cfg! (fn-like)  -> returns closure
// ======================

/// Example:
/// ```ignore
/// let cfg = client_cfg!(timeout = Duration::from_secs(5), no_proxy());
/// let builder = cfg(reqwest::ClientBuilder::new());
/// ```
#[proc_macro]
pub fn client_cfg(input: TokenStream) -> TokenStream {
    let items = parse_macro_input!(input as Items);
    let body = to_stmts(items);
    let out = quote! {
        |mut b: ::common_http::reqwest::ClientBuilder| {
            #body
            b
        }
    };
    out.into()
}

// ======================
// #[client_defaults(...)] (attribute-like)  -> registers module's __cfg via inventory
// ======================

/// syn 2 attribute arg parsing using `syn::meta::ParseNestedMeta`.
///
/// Supported args:
/// - `priority = <int>`
/// - `scope = "<str>"`
///
/// Attach to a module that defines:
/// ```ignore
/// pub fn __cfg(b: common_http::reqwest::ClientBuilder) -> common_http::reqwest::ClientBuilder
/// ```
/// The function will be registered into `inventory` owned by the crate `common_http`.
#[proc_macro_attribute]
pub fn client_defaults(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse attribute args with the new syn 2 meta parser.
    let mut priority: i32 = 0;
    let mut scope: Option<String> = None;

    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("priority") {
            let v: LitInt = meta.value()?.parse()?;
            priority = v.base10_parse::<i32>()?;
            Ok(())
        } else if meta.path.is_ident("scope") {
            let v: LitStr = meta.value()?.parse()?;
            scope = Some(v.value());
            Ok(())
        } else {
            Err(meta.error("unsupported argument (expected `priority = <int>` or `scope = \"...\"`)"))
        }
    });

    // Convert proc_macro::TokenStream to proc_macro2 for the parser.
    if let Err(e) = parser.parse2(proc_macro2::TokenStream::from(attr)) {
        return e.to_compile_error().into();
    }

    // Parse the annotated item as a module; we expect the user to define `pub fn __cfg(...)`.
    let m = parse_macro_input!(item as ItemMod);
    let mod_ident = &m.ident;
    let content = m
        .content
        .as_ref()
        .map(|(_, items)| quote! { #( #items )* })
        .unwrap_or_default();

    let scope_tokens = if let Some(s) = scope {
        quote!( Some(#s) )
    } else {
        quote!( None )
    };

    // Emit: the module (unchanged), plus an inventory::submit! for common_http::ConfigRecord.
    let out = quote! {
        #[allow(non_snake_case)]
        mod #mod_ident {
            use super::*;
            #content

            ::inventory::submit! {
                #![crate = common_http] // adjust if you rename the common_http crate
                ::common_http::ConfigRecord {
                    priority: #priority,
                    scope: #scope_tokens,
                    apply: __cfg,
                }
            }
        }
    };

    out.into()
}
