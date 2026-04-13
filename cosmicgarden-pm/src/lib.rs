//! Identity related proc-macro(s)…
use proc_macro::TokenStream;
use quote::quote;
use syn::{Attribute, Data, DeriveInput, parse_macro_input};

// enum MemberAccess {
//     Direct,
//     Delegate(proc_macro2::Ident),
// }

// fn detect_member_access(data: &Data, target_field: &str) -> MemberAccess {
//     if let Data::Struct(s) = data {
//         if s.fields.iter().any(|f| f.ident.as_ref().map_or(false, |i| i == target_field)) {
//             return MemberAccess::Delegate(format_ident!("{}", target_field));
//         }
//     }
//     MemberAccess::Direct
// }

fn is_tagged_attr(attr: &Attribute, what: &str, goal: &str) -> bool {
    if attr.path().is_ident(what) {
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

macro_rules! get_tagged_ident {
    ($data:ident, $tag:literal, $name:literal) => {
        $data.fields.iter().find(|f| {
            f.attrs.iter().any(|attr| is_tagged_attr(attr, $tag, $name)) ||
            f.ident.as_ref().map_or(false, |i| i == $name)
        })  .map(|f| f.ident.as_ref().unwrap())
            .expect(&format!("Field '{}' not found in #name", $name))
    };
}

macro_rules! req_field {
    ($data:ident, $field:literal) => {
        $data.fields.iter().find(|f| {
            f.ident.as_ref().map_or(false, |i| i == $field)
        })  .map(|f| f.ident.as_ref().unwrap())
            .expect(&format!("No '{}' field found in #name", $field))
    };
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

/// Generate read-only [IdentityQuery] variant's internals to be reused by [IdentityMut] deriver.
fn generate_identity_impl(input: &DeriveInput) -> proc_macro2::TokenStream {
    let name = &input.ident;
    match &input.data {
        Data::Enum(data) => {
            let ids = enum_getter!(data, id);
            let titles = enum_getter!(data, title);
            
            quote! {
                impl crate::identity::IdentityQuery for #name {
                    fn id<'a>(&'a self) -> &'a str { match self {#(#ids),*} }
                    fn title<'a>(&'a self) -> &'a str { match self {#(#titles),*} }
                }
            }
        },
    
        Data::Struct(data) => {
            let f_id = req_field!(data, "id");
            let f_title = get_tagged_ident!(data, "identity", "title");
            
            quote! {
                impl crate::identity::IdentityQuery for #name {
                    fn id<'a>(&'a self) -> &'a str { &self.#f_id }
                    fn title<'a>(&'a self) -> &'a str { &self.#f_title }
                }
            }
        },
    
        _ => unreachable!("Only for Enum/Struct!")
    }
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
pub fn identity_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    TokenStream::from(generate_identity_impl(&input))
}

/// Derive mutable [IdentityMut] (and read-only [IdentityQuery]).
/// 
/// # Required Fields
/// * `id`: String
/// * `title`: String
/// 
/// # Notes
/// * `#[identity(id)]` can be used to tag any field as "id".
/// * `#[identity(title)]` can be used to tag any field as "title".
#[proc_macro_derive(IdentityMut, attributes(identity))]
pub fn identity_mut_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let base_impl = generate_identity_impl(&input);
    let mut_impl =
    match &input.data {
        Data::Enum(data) => {
            let id_mut = enum_getter!(data, id_mut);
            let set_id = enum_setter!(data, set_id);
            let title_mut = enum_getter!(data, title_mut);
            let set_title = enum_setter!(data, set_title);
            quote! {
                impl crate::identity::IdentityMut for #name {
                    fn id_mut<'a>(&'a mut self) -> &'a mut String { match self {#(#id_mut),*} }
                    fn set_id(&mut self, a: &str) -> Result<(), crate::identity::IdError> { match self {#(#set_id),*} }
                    fn title_mut<'a>(&'a mut self) -> &'a mut String { match self {#(#title_mut),*} }
                    fn set_title(&mut self, a: &str) { match self {#(#set_title),*} }
                }
            }
        }
        Data::Struct(data) => {
            let f_id = req_field!(data, "id");
            let title = get_tagged_ident!(data, "identity", "title");
            quote! {
                impl crate::identity::IdentityMut for #name {
                    fn id_mut<'a>(&'a mut self) -> &'a mut String { &mut self.#f_id }
                    fn set_id(&mut self, a: &str) -> Result<(), crate::identity::IdError> {
                        let pre_checked_id = crate::string::slug::as_id(a)?;
                        let no_uuid = crate::string::uuid::remove_uuid(&pre_checked_id);
                        if crate::identity::HARDCODED_RESERVED.contains(no_uuid.as_str()) {
                            return Err(crate::identity::IdError::ReservedName(no_uuid));
                        }
                        *self.id_mut() = pre_checked_id;
                        Ok(())
                    }
                    fn title_mut<'a>(&'a mut self) -> &'a mut String { &mut self.#title }
                    fn set_title(&mut self, a: &str) { *self.title_mut() = a.into() }
                }
            }
        }
        _=> unreachable!("only enum/struct!")
    };

    TokenStream::from(quote! {
        #base_impl
        #mut_impl
    })
}

/// Derive read-only [Mob] token stream.
fn generate_mob_impl(input: &DeriveInput) -> proc_macro2::TokenStream {
    let name = &input.ident;

    let Data::Struct(data) = &input.data else { panic!("Struct only!"); };

    let hp_field = req_field!(data, "hp");
    let mp_field = req_field!(data, "mp");
    let sn_field = req_field!(data, "sn");
    let san_field = req_field!(data, "san");
    quote! {
        impl crate::mob::traits::Mob for #name {
            fn hp(&self) -> &crate::mob::stat::Stat { &self.#hp_field }
            fn mp(&self) -> &crate::mob::stat::Stat { &self.#mp_field }
            fn sn(&self) -> &crate::mob::stat::Stat { &self.#sn_field }
            fn san(&self) -> &crate::mob::stat::Stat { &self.#san_field }
        }
    }
}

/// Derive read-only [Mob].
#[proc_macro_derive(Mob)]
pub fn mob_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    TokenStream::from(generate_mob_impl(&input))
}

/// Derive read-only [Mob] and mutable [MobMut] both at once.
#[proc_macro_derive(MobMut)]
pub fn mob_mut_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let base_impl = generate_mob_impl(&input);
    let Data::Struct(data) = input.data else { panic!("Struct only!") };
    let hp_field = req_field!(data, "hp");
    let mp_field = req_field!(data, "mp");
    let sn_field = req_field!(data, "sn");
    let san_field = req_field!(data, "san");
    let mut_impl = quote! {
        impl crate::mob::traits::MobMut for #name {
            fn hp_mut(&mut self) -> &mut crate::mob::stat::Stat { &mut self.#hp_field }
            fn mp_mut(&mut self) -> &mut crate::mob::stat::Stat { &mut self.#mp_field }
            fn sn_mut(&mut self) -> &mut crate::mob::stat::Stat { &mut self.#sn_field }
            fn san_mut(&mut self) -> &mut crate::mob::stat::Stat { &mut self.#san_field }
        }
    };
    TokenStream::from(quote! {
        #base_impl
        #mut_impl
    })
}

/// Generate r/o [Itemized]'s token stream.
fn generate_itemized_impl(input: &DeriveInput) -> proc_macro2::TokenStream {
    let name = &input.ident;
    match &input.data {
        Data::Enum(data) => {
            let sizes = enum_getter!(data, size);
            quote! {
                impl crate::item::Itemized for #name {
                    fn size(&self) -> crate::item::StorageSpace { match self {#(#sizes),*}}
                }
            }
        }

        Data::Struct(data) => {
            let size = get_tagged_ident!(data, "measurement", "size");
            quote! {
                impl crate::item::Itemized for #name {
                    fn size(&self) -> crate::item::StorageSpace { self.#size }
                }
            }
        }

        _ => unreachable!("Go away…")
    }
}

/// Query [Itemized] derive.
#[proc_macro_derive(Itemized, attributes(measurement))]
pub fn itemized_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    TokenStream::from(generate_itemized_impl(&input))
}

/// Mutating [ItemizedMut] and [Itemized] in one.
#[proc_macro_derive(ItemizedMut, attributes(measurement))]
pub fn itemized_mut_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let base_impl = generate_itemized_impl(&input);
    let mut_impl = match &input.data
    {
        Data::Enum(data) => {
            let set_sizes = enum_setter!(data, set_size);

            quote! {
                impl crate::item::ItemizedMut for #name {
                    fn set_size(&mut self, a: crate::item::StorageSpace) -> bool { match self {#(#set_sizes),*}}
                }
            }
        }
        
        Data::Struct(data) => {
            let size = get_tagged_ident!(data, "measurement", "size");
            quote! {
                impl crate::item::ItemizedMut for #name {
                    fn set_size(&mut self, a: crate::item::StorageSpace) -> bool { self.#size = a; true }
                }
            }
        }

        _ => unreachable!("Go away…")
    };

    TokenStream::from(quote! {
        #base_impl
        #mut_impl
    })
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

/// Generate [Describable] impl.
fn generate_describable_impl(input: &DeriveInput) -> proc_macro2::TokenStream {
    let name = &input.ident;
    match &input.data {
        Data::Enum(data) => {
            let descs = enum_getter!(data, desc);
            quote! {
                impl crate::string::description::Describable for #name {
                    fn desc<'a>(&'a self) -> &'a str { match self {#(#descs),*}}
                }
            }
        }

        Data::Struct(data) => {
            let f_desc = get_tagged_ident!(data, "description", "desc");
            quote! {
                impl crate::string::description::Describable for #name {
                    fn desc<'a>(&'a self) -> &'a str { &self.#f_desc }
                }
            }
        }

        _ => unreachable!("Go away…")
    }
}

/// Derive [Describable].
#[proc_macro_derive(Describable)]
pub fn describable_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    TokenStream::from(generate_describable_impl(&input))
}

/// Derive [DescribableMut] (and [Describable]).
#[proc_macro_derive(DescribableMut)]
pub fn describable_mut_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let base_impl = generate_describable_impl(&input);
    let mut_impl = match &input.data
    {
        Data::Enum(data) => {
            let set_descs = enum_setter!(data, set_desc);
            quote! {
                impl crate::string::description::DescribableMut for #name {
                    fn set_desc(&mut self, a: &str) -> bool { match self {#(#set_descs),*}}
                }
            }
        }

        Data::Struct(data) => {
            let f_desc = get_tagged_ident!(data, "description", "desc");
            quote! {
                impl crate::string::description::DescribableMut for #name {
                    fn set_desc(&mut self, a: &str) -> bool {
                        self.desc = a.to_string();
                        true
                    }
                }
            }
        }

        _ => unreachable!("Go away…")
    };

    TokenStream::from(quote! {
        #base_impl
        #mut_impl
    })
}

/// Generate read-only [Owned] variant's internals to be reused by [OwnedMut] deriver.
fn generate_owned_impl(input: &DeriveInput) -> proc_macro2::TokenStream {
    let name = &input.ident;

    match &input.data {
        Data::Enum(data) => {
            let owner_ids = enum_getter!(data, owner);
            let last_user_ids = enum_getter!(data, last_user);
            let sources = enum_getter!(data, source);
            
            quote! {
                impl crate::item::ownership::Owned for #name {
                    fn owner(&self) -> Option<String> { match self {#(#owner_ids),*}}
                    fn last_user(&self) -> Option<String> { match self {#(#last_user_ids),*}}
                    fn source(&self) -> crate::item::ownership::ItemSource { match self {#(#sources),*}}
                }
            }
        },
    
        Data::Struct(data) => {
            let has_owner_field = data.fields.iter().any(|f| {
                f.ident.as_ref().map(|i| i == "owner").unwrap_or(false)
            });

            let (o_body, l_body, s_body) = if has_owner_field {
            (
                quote! { self.owner.owner() },
                quote! { self.owner.last_user() },
                quote! { self.owner.source() },
            )
            } else {
            (
                quote! { self.owner_id.clone() },
                quote! { self.last_user_id.clone() },
                quote! { self.source.clone() },
            )
            };

            quote! {
                impl crate::item::ownership::Owned for #name {
                    fn owner(&self) -> Option<String> { #o_body }
                    fn last_user(&self) -> Option<String> { #l_body }
                    fn source(&self) -> crate::item::ownership::ItemSource { #s_body }
                }
            }
        },
    
        _ => panic!("Only for Enum/Struct!")
    }
}

/// Derive [Owned].
#[proc_macro_derive(Owned)]
pub fn owned_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    TokenStream::from(generate_owned_impl(&input))
}

/// Derive [OwnedMut] and [Owned].
#[proc_macro_derive(OwnedMut)]
pub fn owned_mut_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let base_impl = generate_owned_impl(&input);
    let mut_impl = match &input.data {
        Data::Enum(data) => {
            let set_owner_ids = enum_setter!(data, change_owner);
            let set_last_user_ids = enum_setter!(data, set_last_user);
            let set_sources = enum_setter_3!(data, set_source);
            quote! {
                impl crate::item::ownership::OwnedMut for #name {
                    fn change_owner(&mut self, a: &str) { match self {#(#set_owner_ids),*}}
                    fn set_last_user(&mut self, a: &str) -> Result<(), crate::identity::IdError> { match self {#(#set_last_user_ids),*}}
                    fn set_source(&mut self, a: &str, b: &str, c: crate::item::ownership::ItemSource) -> Result<(), crate::item::ownership::ItemSourceError> { match self {#(#set_sources),*}}
                }
            }
        }
        
        Data::Struct(data) => {
            let has_owner_field = data.fields.iter().any(|f| {
                f.ident.as_ref().map(|i| i == "owner").unwrap_or(false)
            });

            let (o_body, l_body, s_body) = if has_owner_field {(
                quote! { self.owner.change_owner(a) },
                quote! { self.owner.set_last_user(a) },
                quote! { self.owner.set_source(a,b,c) },
            )} else {(
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
                        return Err(crate::item::ownership::ItemSourceError::Rejected);
                    }
                    self.source = c;
                    Ok(())
                }
            )};
            quote! {
                impl crate::item::ownership::OwnedMut for #name {
                    fn change_owner(&mut self, a: &str) { #o_body }
                    fn set_last_user(&mut self, a: &str) -> Result<(), crate::identity::IdError> { #l_body }
                    fn set_source(&mut self, a: &str, b: &str, c: crate::item::ownership::ItemSource) -> Result<(), crate::item::ownership::ItemSourceError> { #s_body }
                }
            }
        }

        _ => unimplemented!("Only for Enum/Struct!")
    };

    TokenStream::from(quote! {
        #base_impl
        #mut_impl
    })
}
