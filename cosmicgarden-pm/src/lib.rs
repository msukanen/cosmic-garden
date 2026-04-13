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

/// Derive read-only [IdentityQuery].
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
            quote! { Self::#variant_name(inner) => inner.$getter(a) }
        })
    };
}

macro_rules! enum_setter {
    ($data:ident, $setter:ident) => {
        $data.variants.iter().map(|v| {
            let variant_name = &v.ident;
            quote! { Self::#variant_name(inner) => inner.$setter(a) }
        })
    };
}

macro_rules! enum_setter_3 {
    ($data:ident, $setter:ident) => {
        $data.variants.iter().map(|v| {
            let variant_name = &v.ident;
            quote! { Self::#variant_name(inner) => inner.$setter(a,b,c) }
        })
    };
}

/// Derive mutable [IdentityMut] and read-only [IdentityQuery] both at once.
#[proc_macro_derive(IdentityMut, attributes(identity))]
pub fn identity_mut_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    if let Data::Enum(data) = input.data {
        let id_muts = enum_getter!(data, id_mut);
        let set_ids = enum_setter!(data, set_id);
        let title_muts = enum_getter!(data, title_mut);
        let set_titles = enum_setter!(data, set_title);
        
        let ids = enum_getter!(data, id);
        let titles = enum_getter!(data, title);

        TokenStream::from(quote! {
            impl crate::identity::IdentityMut for #name {
                fn id_mut<'a>(&'a mut self) -> &'a mut String { match self {#(#id_muts),*} }
                fn set_id(&mut self, a: &str) -> Result<(), crate::identity::IdError> { match self {#(#set_ids),*} }
                fn title_mut<'a>(&'a mut self) -> &'a mut String { match self {#(#title_muts),*} }
                fn set_title(&mut self, a: &str) { match self {#(#set_titles),*} }
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

/// Derive read-only [Mob].
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

/// Derive read-only [Mob] and mutable [MobMut] both at once.
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

/// Query [Itemized] derive.
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

/// Mutating [ItemizedMut] and [Itemized] in one.
#[proc_macro_derive(ItemizedMut)]
pub fn itemized_mut_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    if let Data::Enum(data) = input.data {
        let sizes = enum_getter!(data, size);
        let set_sizes = enum_setter!(data, set_size);

        TokenStream::from(quote! {
            impl crate::item::Itemized for #name {
                fn size(&self) -> crate::item::StorageSpace { match self {#(#sizes),*}}
            }
            impl crate::item::ItemizedMut for #name {
                fn set_size(&mut self, a: crate::item::StorageSpace) -> bool { match self {#(#set_sizes),*}}
            }
        })
    } else {
        TokenStream::from(quote! {
            impl crate::item::Itemized for #name {
                fn size(&self) -> crate::item::StorageSpace { self.size }
            }
            impl crate::item::ItemizedMut for #name {
                fn set_size(&mut self, a: crate::item::StorageSpace) -> bool { self.size = a; true }
            }
        })
    }
}

/// [Storage] related derive.
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
        let take_bys = enum_getter_w_arg!(data, take_by_name);
        let find_id_by_names = enum_getter_w_arg!(data, find_id_by_name);
        let ejects = enum_getter!(data, eject_all);

        TokenStream::from(quote! {
            impl crate::item::container::Storage for #name {
                fn can_hold(&self, a: &crate::item::Item) -> Result<(), crate::item::StorageQueryError>
                    { match self {#(#can_holds),*}}
                fn max_space(&self) -> crate::item::StorageSpace { match self {#(#max_spaces),*}}
                fn required_space(&self) -> crate::item::StorageSpace { match self {#(#req_spaces),*}}
                fn space(&self) -> crate::item::StorageSpace { match self {#(#spaces),*}}
                fn try_insert(&mut self, a: crate::item::Item) -> Result<(), crate::item::StorageError>
                    { match self {#(#try_inserts),*}}
                fn contains(&self, a: &str) -> bool { match self {#(#contains),*}}
                fn peek_at(&self, a: &str) -> Option<&Item> { match self {#(#peek_ats),*}}
                fn take(&mut self, a: &str) -> Option<Item> { match self {#(#takes),*}}
                fn take_by_name(&mut self, a: &str) -> Option<Item> { match self {#(#take_bys),*}}
                fn find_id_by_name(&self, a: &str) -> Option<String> { match self {#(#find_id_by_names),*}}
                fn eject_all(&mut self) -> Option<Vec<Item>> { match self {#(#ejects),*}}
            }
        })
    } else {
        panic!("No no! Enum only!")
    }
}

/// Derive [Describable].
#[proc_macro_derive(Describable)]
pub fn describable_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    if let Data::Enum(data) = input.data {
        let descs = enum_getter!(data, desc);
        let set_descs = enum_setter!(data, set_desc);
        TokenStream::from(quote! {
            impl crate::string::description::Describable for #name {
                fn desc<'a>(&'a self) -> &'a str => { match self {#(#descs),*}}
                fn set_desc(&mut self, a: &str) -> bool { match self {#(#set_descs),*}}
            }
        })
    } else {
        TokenStream::from(quote! {
            impl crate::string::description::Describable for #name {
                fn desc<'a>(&'a self) -> &'a str { &self.desc }
                fn set_desc(&mut self, a: &str) -> bool {
                    self.desc = a.to_string();
                    true
                }
            }
        })
    }
}

/// Derive [DescribableMut] (and [Describable]).
#[proc_macro_derive(DescribableMut)]
pub fn describable_mut_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    if let Data::Enum(data) = input.data {
        let descs = enum_getter!(data, desc);
        let set_descs = enum_setter!(data, set_desc);
        TokenStream::from(quote! {
            impl crate::string::description::Describable for #name {
                fn desc<'a>(&'a self) -> &'a str => { match self {#(#descs),*}}
            }
            impl crate::string::description::DescribableMut for #name {
                fn set_desc(&mut self, a: &str) -> bool { match self {#(#set_descs),*}}
            }
        })
    } else {
        TokenStream::from(quote! {
            impl crate::string::description::Describable for #name {
                fn desc<'a>(&'a self) -> &'a str { &self.desc }
            }
            impl crate::string::description::DescribableMut for #name {
                fn set_desc(&mut self, a: &str) -> bool {
                    self.desc = a.to_string();
                    true
                }
            }
        })
    }
}

/// Derive [Owned].
// #[proc_macro_derive(Owned)]
// pub fn owned_derive(input: TokenStream) -> TokenStream {
//     let input = parse_macro_input!(input as DeriveInput);
//     let name = input.ident;

//     if let Data::Enum(data) = input.data {
//         let owner_ids = enum_getter!(data, owner_id);
//         let last_user_ids = enum_getter!(data, last_user_id);
//         let sources = enum_getter!(data, source);
        
//         TokenStream::from(quote! {
//             impl crate::item::ownership::Owned for #name {
//                 fn owner_id(&self) -> Option<String> => { match self {#(#owner_ids),*}}
//                 fn last_user_id(&self) -> Option<String> => { match self {#(#last_user_ids),*}}
//                 fn source(&self) -> crate::item::ownership::ItemSource => { match self {#(#sources),*}}
//             }
//         })
//     } else {
//         TokenStream::from(quote! {
//             impl crate::item::ownership::Owned for #name {
//                 fn owner_id(&self) -> Option<String> { &self.owner_id }
//                 fn last_user_id(&self) -> Option<String> { &self.last_user_id }
//                 fn source(&self) -> crate::item::ownership::ItemSource { self.source.clone() }
//             }
//         })
//     }
// }

/// Derive [OwnedMut] and [Owned].
#[proc_macro_derive(OwnedMut)]
pub fn owned_mut_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    if let Data::Enum(data) = input.data {
        let owner_ids = enum_getter!(data, owner);
        let last_user_ids = enum_getter!(data, last_user);
        let sources = enum_getter!(data, source);
        let set_owner_ids = enum_setter!(data, change_owner);
        let set_last_user_ids = enum_setter!(data, set_last_user);
        let set_sources = enum_setter_3!(data, set_source);
        
        TokenStream::from(quote! {
            impl crate::item::ownership::Owned for #name {
                fn owner(&self) -> Option<String> { match self {#(#owner_ids),*}}
                fn last_user(&self) -> Option<String> { match self {#(#last_user_ids),*}}
                fn source(&self) -> crate::item::ownership::ItemSource { match self {#(#sources),*}}
            }
            impl crate::item::ownership::OwnedMut for #name {
                fn change_owner(&mut self, a: &str) { match self {#(#set_owner_ids),*}}
                fn set_last_user(&mut self, a: &str) -> Result<(), crate::identity::IdError> { match self {#(#set_last_user_ids),*}}
                fn set_source(&mut self, a: &str, b: &str, c: crate::item::ownership::ItemSource) -> Result<(), crate::item::ownership::ItemSourceError> { match self {#(#set_sources),*}}
            }
        })
    } else if let Data::Struct(ref data) = input.data {
        let has_owner_field = data.fields.iter().any(|f| {
            f.ident.as_ref().map(|i| i == "owner").unwrap_or(false)
        });

        let (owner_id_body, last_user_body, source_body, change_owner_body, set_last_user_body, set_source_body) = if has_owner_field {
            (
                quote! { self.owner.owner() },
                quote! { self.owner.last_user() },
                quote! { self.owner.source() },
                quote! { self.owner.change_owner(a) },
                quote! { self.owner.set_last_user(a) },
                quote! { self.owner.set_source(a,b,c) },
            )
        } else {
            (
                quote! { self.owner_id.clone() },
                quote! { self.last_user_id.clone() },
                quote! { self.source.clone() },
                quote! {// change_owner
                    if let Some(ref mut prev_owner) = self.owner_id {
                        log::trace!("Changing ownership from '{}' to '{}'", prev_owner, a);
                        *prev_owner = a.to_string();
                    } else {
                        self.owner_id = a.to_string().into();
                    }
                },
                quote! {// set_last_user
                    crate::string::slug::is_id(a)?;
                    self.last_user_id = a.to_string().into();
                    Ok(())
                },
                quote! {// set_source
                    if let crate::item::ownership::ItemSource::Blueprint = c {
                        log::warn!("Hol'up! Rejecting demotion of '{}' to blueprint by '{}'.", a, b);
                        return Err(ItemSourceError::Rejected);
                    }
                    self.source = c;
                    Ok(())
                }
            )

        };

        TokenStream::from(quote! {
            impl crate::item::ownership::Owned for #name {
                fn owner(&self) -> Option<String> { #owner_id_body }
                fn last_user(&self) -> Option<String> { #last_user_body }
                fn source(&self) -> crate::item::ownership::ItemSource { #source_body }
            }

            impl crate::item::ownership::OwnedMut for #name {
                fn change_owner(&mut self, a: &str) { #change_owner_body }
                fn set_last_user(&mut self, a: &str) -> Result<(), crate::identity::IdError> { #set_last_user_body }
                fn set_source(&mut self, a: &str, b: &str, c: crate::item::ownership::ItemSource) -> Result<(), crate::item::ownership::ItemSourceError> { #set_source_body }
            }
        })
    } else { panic!("Only for Enum/Struct!") }
}
