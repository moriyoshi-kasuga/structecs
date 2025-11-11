use std::{collections::HashMap, iter::chain};

use proc_macro2::TokenStream;
use syn::{DeriveInput, Ident, Type, punctuated::Punctuated};

enum Metadata<'a> {
    Offset0 {
        target_type: &'a Ident,
    },
    Nested {
        field_ident: &'a Ident,
        target_type: &'a Type,
    },
}

pub(crate) fn internal_derive(input: DeriveInput) -> syn::Result<TokenStream> {
    let target_fields: Vec<Ident> = input.attrs.iter().try_fold(Vec::new(), |mut acc, attr| {
        if !attr.path().is_ident("extractable") {
            return Ok::<_, syn::Error>(acc);
        }
        let target_fields: Punctuated<Ident, syn::Token![,]> =
            attr.parse_args_with(Punctuated::parse_terminated)?;
        acc.extend(target_fields);
        Ok(acc)
    })?;

    let offset0 = Metadata::Offset0 {
        target_type: &input.ident,
    };

    let data_struct = match &input.data {
        _ if target_fields.is_empty() => {
            return expand(vec![offset0], &input);
        }
        syn::Data::Struct(data) => data,
        _ => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "Extractable fields can only be derived for structs.",
            ));
        }
    };

    let fields = match data_struct.fields {
        syn::Fields::Named(ref fields_named) => &fields_named.named,
        _ => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "Extractable can only be derived for structs with named fields. Cannot be derived for tuple structs or unit structs.",
            ));
        }
    };

    let field_idents: HashMap<&Ident, &Type> = fields
        .iter()
        .filter_map(|field| {
            let field_ident = field.ident.as_ref()?;
            let field_type = &field.ty;
            Some((field_ident, field_type))
        })
        .collect();

    let attrs = target_fields
        .iter()
        .map(|field_ident| {
            let target_type = field_idents.get(field_ident).ok_or_else(|| {
                syn::Error::new_spanned(
                    field_ident,
                    format!("Field '{}' not found in struct.", field_ident),
                )
            })?;

            Ok(Metadata::Nested {
                field_ident,
                target_type,
            })
        })
        .collect::<Result<Vec<Metadata>, syn::Error>>()?;

    expand(chain([offset0], attrs).collect(), &input)
}

fn expand(attr: Vec<Metadata<'_>>, input: &DeriveInput) -> syn::Result<TokenStream> {
    let struct_name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let metadata_list = attr
        .iter()
        .map(|attr| match attr {
            Metadata::Offset0 { target_type } => {
                quote::quote! {
                    structecs::ExtractionMetadata::new::<#target_type>(0),
                }
            }
            Metadata::Nested {
                field_ident,
                target_type,
            } => {
                quote::quote! {
                    structecs::ExtractionMetadata::new_nested::<#target_type>(
                        core::mem::offset_of!(#struct_name, #field_ident),
                        #target_type::METADATA_LIST,
                    ),
                }
            }
        })
        .collect::<TokenStream>();

    Ok(quote::quote! {
        impl #impl_generics structecs::Extractable for #struct_name #ty_generics #where_clause {
            const METADATA_LIST: &'static [structecs::ExtractionMetadata] = &[
                #metadata_list
            ];

            #[cfg(debug_assertions)]
            const IDENTIFIER: &'static str = {
                const MODULE_PATH: &str = module_path!();
                const STRUCT_NAME: &str = stringify!(#struct_name);
                const TOTAL: usize = MODULE_PATH.len() + 2 + STRUCT_NAME.len();
                const FULL_IDENTIFIER_BYTES: [u8; TOTAL] =
                    structecs::__private::concat_str::<TOTAL>(
                        MODULE_PATH,
                        STRUCT_NAME,
                    );
                unsafe { core::str::from_utf8_unchecked(&FULL_IDENTIFIER_BYTES) }
            };
        }

        structecs::__private::submit! {
            structecs::ExtractableType::new::<#struct_name>()
        }
    })
}
