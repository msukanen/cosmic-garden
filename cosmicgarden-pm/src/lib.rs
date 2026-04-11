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

/// Derive read-only `IdentityQuery`.
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

    TokenStream::from(quote! {
        impl crate::identity::IdentityQuery for #name {
            fn id(&self) -> &str { &self.#id_ident }
            fn title(&self) -> &str { &self.#title_ident }
        }
    })
}

macro_rules! enum_getter {
    ($data:ident, $getter:ident) => {
        $data.variants.iter().map(|v| {
            let variant_name = &v.ident;
            quote! { Self::#variant_name(inner) => inner.$getter() }
        })
    };
}

macro_rules! enum_getter_w_arg {
    ($data:ident, $getter:ident) => {
        $data.variants.iter().map(|v| {
            let variant_name = &v.ident;
            quote! { Self::#variant_name(inner) => inner.$getter(value) }
        })
    };
}

macro_rules! enum_setter_w_result {
    ($data:ident, $setter:ident) => {
        $data.variants.iter().map(|v| {
            let variant_name = &v.ident;
            quote! { Self::#variant_name(inner) => inner.$setter(value) }
        })
    };
}

macro_rules! enum_setter {
    ($data:ident, $setter:ident) => {
        $data.variants.iter().map(|v| {
            let variant_name = &v.ident;
            quote! { Self::#variant_name(inner) => inner.$setter(value) }
        })
    };
}

/// Derive mutable `IdentityMut` and read-only `IdentityQuery` both at once.
#[proc_macro_derive(IdentityMut, attributes(identity))]
pub fn identity_mut_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    if let Data::Enum(data) = input.data {
        let id_muts = enum_getter!(data, id_mut);
        let set_ids = enum_setter_w_result!(data, set_id);
        let title_muts = enum_getter!(data, title_mut);
        let set_titles = enum_setter!(data, set_title);
        let ids = enum_getter!(data, id);
        let titles = enum_getter!(data, title);

        TokenStream::from(quote! {
            impl crate::identity::IdentityMut for #name {
                fn id_mut<'a>(&'a mut self) -> &'a mut String { match self {#(#id_muts),*} }
                fn set_id(&mut self, value: &str) -> Result<(), crate::identity::IdError> { match self {#(#set_ids),*} }
                fn title_mut<'a>(&'a mut self) -> &'a mut String { match self {#(#title_muts),*} }
                fn set_title(&mut self, value: &str) { match self {#(#set_titles),*} }
            }

            impl crate::identity::IdentityQuery for #name {
                fn id<'a>(&'a self) -> &'a str { match self {#(#ids),*} }
                fn title<'a>(&'a self) -> &'a str { match self {#(#titles),*} }
            }
        })
    } else {
        let fields = get_identity_fields!(input);
        let id_ident = &(get_identity_ident!(fields, "id").ident);
        let title_ident = &(get_identity_ident!(fields, "title").ident);

        TokenStream::from(quote! {
            impl crate::identity::IdentityQuery for #name {
                fn id(&self) -> &str { &self.#id_ident }
                fn title(&self) -> &str { &self.#title_ident }
            }

            impl crate::identity::IdentityMut for #name {
                fn id_mut(&mut self) -> &mut String { &mut self.#id_ident }
                fn title_mut(&mut self) -> &mut String { &mut self.#title_ident }
            }
        })
    }
}

/// Derive read-only `Mob`.
#[proc_macro_derive(Mob, attributes(identity))]
pub fn mob_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let fields = get_identity_fields!(input);
    let hp_field = &(get_identity_ident!(fields, "hp").ident);
    let mp_field = &(get_identity_ident!(fields, "mp").ident);
    let sn_field = &(get_identity_ident!(fields, "sn").ident);
    let san_field = &(get_identity_ident!(fields, "san").ident);

    TokenStream::from(quote! {
        impl crate::mob::traits::Mob for #name {
            fn hp(&self) -> &Stat { &self.#hp_field }
            fn mp(&self) -> &Stat { &self.#mp_field }
            fn sn(&self) -> &Stat { &self.#sn_field }
            fn san(&self) -> &Stat { &self.#san_field }
        }
    })
}

/// Derive read-only `Mob` and mutable `MobMut` both at once.
#[proc_macro_derive(MobMut, attributes(identity))]
pub fn mob_mut_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let fields = get_identity_fields!(input);
    let hp_field = &(get_identity_ident!(fields, "hp").ident);
    let mp_field = &(get_identity_ident!(fields, "mp").ident);
    let sn_field = &(get_identity_ident!(fields, "sn").ident);
    let san_field = &(get_identity_ident!(fields, "san").ident);

    TokenStream::from(quote! {
        impl crate::mob::traits::Mob for #name {
            fn hp(&self) -> &Stat { &self.#hp_field }
            fn mp(&self) -> &Stat { &self.#mp_field }
            fn sn(&self) -> &Stat { &self.#sn_field }
            fn san(&self) -> &Stat { &self.#san_field }
        }

        impl crate::mob::traits::MobMut for #name {
            fn hp_mut(&mut self) -> &mut Stat { &mut self.#hp_field }
            fn mp_mut(&mut self) -> &mut Stat { &mut self.#mp_field }
            fn sn_mut(&mut self) -> &mut Stat { &mut self.#sn_field }
            fn san_mut(&mut self) -> &mut Stat { &mut self.#san_field }
        }
    })
}

#[proc_macro_derive(Itemized)]
pub fn itemized_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    if let Data::Enum(data) = input.data {
        let sizes = enum_getter!(data, size);

        TokenStream::from(quote! {
            impl crate::item::Itemized for #name {
                fn size(&self) -> crate::item::StorageSpace { match self {#(#sizes),*}}
            }
        })
    } else {
        TokenStream::from(quote! {
            impl crate::item::Itemized for #name {
                fn size(&self) -> crate::item::StorageSpace { self.size }
            }
        })
    }
}

#[proc_macro_derive(ItemizedMut)]
pub fn itemized_mut_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    if let Data::Enum(data) = input.data {
        let sizes = enum_getter!(data, size);
        let set_sizes = enum_getter!(data, set_size);

        TokenStream::from(quote! {
            impl crate::item::Itemized for #name {
                fn size(&self) -> crate::item::StorageSpace { match self {#(#sizes),*}}
            }
            impl crate::item::ItemizedMut for #name {
                fn set_size(&mut self, value: crate::item::StorageSpace) -> bool { match self {#(#set_sizes),*}}
            }
        })
    } else {
        TokenStream::from(quote! {
            impl crate::item::Itemized for #name {
                fn size(&self) -> crate::item::StorageSpace { self.size }
            }
            impl crate::item::ItemizedMut for #name {
                fn set_size(&mut self, value: crate::item::StorageSpace) -> bool { self.size = value; true }
            }
        })
    }
}

#[proc_macro_derive(Storage)]
pub fn storage_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    if let Data::Enum(data) = input.data {
        let can_holds = enum_getter_w_arg!(data, can_hold);
        let max_spaces = enum_getter!(data, max_space);
        let req_spaces = enum_getter!(data, required_space);
        let spaces = enum_getter!(data, space);
        let try_inserts = enum_getter_w_arg!(data, try_insert);
        let contains = enum_getter_w_arg!(data, contains);
        let peek_ats = enum_getter_w_arg!(data, peek_at);
        let takes = enum_getter_w_arg!(data, take);

        TokenStream::from(quote! {
            impl crate::item::container::Storage for #name {
                fn can_hold(&self, value: &crate::item::Item) -> Result<bool, crate::item::StorageError> { match self {#(#can_holds),*}}
                fn max_space(&self) -> crate::item::StorageSpace { match self {#(#max_spaces),*}}
                fn required_space(&self) -> crate::item::StorageSpace { match self {#(#req_spaces),*}}
                fn space(&self) -> crate::item::StorageSpace { match self {#(#spaces),*}}
                fn try_insert(&mut self, value: crate::item::Item) -> Result<(), crate::item::StorageError> { match self {#(#try_inserts),*}}
                fn contains(&self, value: &str) -> bool { match self {#(#contains),*}}
                fn peek_at(&self, value: &str) -> Option<&Item> { match self {#(#peek_ats),*}}
                fn take(&mut self, value: &str) -> Option<Item> { match self {#(#takes),*}}
            }
        })
    } else {
        panic!("No no! Enum only!")
    }
}
