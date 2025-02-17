use cosmwasm_std::testing::MockApi;

use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

#[proc_macro]
pub fn addr(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::LitStr).value();
    let addr = MockApi::default().addr_make(input.as_str()).to_string();
    TokenStream::from(quote! {#addr})
}
