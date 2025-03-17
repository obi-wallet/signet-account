extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemEnum, ItemStruct};

#[proc_macro_attribute]
pub fn uniserde(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as proc_macro2::TokenStream);

    // Try to parse as struct, then as enum
    if syn::parse2::<ItemStruct>(input.clone()).is_ok()
        || syn::parse2::<ItemEnum>(input.clone()).is_ok()
    {
        return handle_struct_or_enum(input);
    };

    panic!("The attribute can only be used with structs or enums");
}

fn handle_struct_or_enum(item: proc_macro2::TokenStream) -> TokenStream {
    let expanded = quote! {
        #[cfg_attr(feature = "cosmwasm", cw_serde)]
        #[cfg_attr(
            all(feature = "secretwasm", not(feature = "cosmwasm")),
            derive(
                serde::Serialize,
                serde::Deserialize,
                Clone,
                Debug,
                Eq,
                PartialEq,
                schemars::JsonSchema
            )
        )]
        #[cfg_attr(
            all(feature = "secretwasm", not(feature = "cosmwasm")),
            serde(rename_all = "snake_case")
        )]
        #item
    };

    TokenStream::from(expanded)
}
