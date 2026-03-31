//! Identity related proc-macro(s)…
use proc_macro::TokenStream;
use quote::quote;
use syn::{Attribute, Data, DataStruct, DeriveInput, Fields, FieldsNamed, parse_macro_input};

fn is_identity_attr(attr: &Attribute, goal: &str) -> bool {
    if attr.path().is_ident("identity") {
        let mut found = false;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident(goal) {
                found = true;
            }
            Ok(())
        });
        return found;
    }

    false
}

macro_rules! get_identity_ident {
    ($fields:ident, $name:literal) => {
        $fields.iter().find(|f| {
            f.attrs.iter().any(|attr| is_identity_attr(attr, $name)) ||
            f.ident.as_ref().map_or(false, |i| i == $name)
        }).expect(&format!("No '{}' field or #[identity({})] found!", $name, $name))
    };
}

macro_rules! get_identity_fields {
    ($input:ident) => {
        if let Data::Struct(DataStruct {
            fields: Fields::Named(FieldsNamed { named, .. }), ..
        }) = $input.data {
            named
        } else {
            panic!("IdentityQuery only works on structs with named fields!")
        }
    };
}

/// Derive `IdentityQuery`.
/// 
/// # Required Fields
/// * `id`: String
/// * `title`: String
/// 
/// # Notes
/// * `#[identity(id)]` can be used to tag any field as "id".
/// * `#[identity(title)]` can be used to tag any field as "title".
#[proc_macro_derive(IdentityQuery, attributes(identity))]
pub fn identity_query_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let fields = get_identity_fields!(input);
    let id_ident = &(get_identity_ident!(fields, "id").ident);
    let title_ident = &(get_identity_ident!(fields, "title").ident);

    let expanded = quote! {
        impl crate::identity::IdentityQuery for #name {
            fn id(&self) -> &str { &self.#id_ident }
            fn title(&self) -> &str { &self.#title_ident }
        }
    };

    TokenStream::from(expanded)
}

/// Derive `IdentityMut`.
#[proc_macro_derive(IdentityMut, attributes(identity))]
pub fn identity_mut_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let fields = get_identity_fields!(input);
    let id_ident = &(get_identity_ident!(fields, "id").ident);
    let title_ident = &(get_identity_ident!(fields, "title").ident);

    let expanded = quote! {
        impl crate::identity::IdentityMut for #name {
            fn id_mut(&mut self) -> &mut String { &mut self.#id_ident }
            fn title_mut(&mut self) -> &mut String { &mut self.#title_ident }
        }
    };

    TokenStream::from(expanded)
}
