use syn::{DeriveInput, parse_macro_input};

mod extractable;

#[proc_macro_derive(Extractable, attributes(extractable))]
pub fn extend_macro_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    extractable::internal_derive(parse_macro_input!(input as DeriveInput))
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
