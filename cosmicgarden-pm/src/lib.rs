//! Garden's proc-macro(s)…
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{Attribute, Data, DataEnum, DeriveInput, Fields, Ident, parse_macro_input};

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

fn gen_container_match(data: &DataEnum, method: &Ident, num_arg: u32) -> Vec<proc_macro2::TokenStream> {
    data.variants.iter().map(|variant| {
        let arg = match num_arg {
            0 => quote!(),
            1 => quote!(a),
            2 => quote!(a,b),
            3 => quote!(a,b,c),
            _ => quote!(a,b,c,d),
        };
        let var_ident = &variant.ident;
        match &variant.fields {
            Fields::Unnamed(_) => {
                quote! {
                    Self::#var_ident(inner) => inner.#method(#arg)
                }
            }

            Fields::Named(_) => {
                quote! {
                    Self::#var_ident { loot, ..} => loot.#method(#arg)
                }
            }

            Fields::Unit => { quote! { Self::#var_ident => panic!("No Storage for weird stuff!") }}
        }
    }).collect()
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

macro_rules! maybe_field {
    ($data:ident, $field:literal) => {
        $data.fields.iter().find(|f| {
            f.ident.as_ref().map_or(false, |i| i == $field)
        })  .map(|f| f.ident.as_ref().unwrap())
    };
}

/// Generate read-only [IdentityQuery] variant's internals.
fn generate_identity_impl(input: &DeriveInput) -> proc_macro2::TokenStream {
    let name = &input.ident;
    match &input.data {
        Data::Enum(data) => {
            let ids = gen_container_match(&data, &format_ident!("id"), 0);
            let titles = gen_container_match(&data, &format_ident!("title"), 0);
            
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
            let set_id = gen_container_match(&data, &format_ident!("set_id"), 2);
            let title_mut = gen_container_match(&data, &format_ident!("title_mut"), 0);
            let set_title = gen_container_match(&data, &format_ident!("set_title"), 1);
            quote! {
                impl crate::identity::IdentityMut for #name {
                    fn set_id(&mut self, a: &str, b: bool) -> Result<(), crate::identity::IdError> { match self {#(#set_id),*} }
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
                    fn set_id(&mut self, a: &str, b: bool) -> Result<(), crate::identity::IdError> {
                        use crate::identity::uniq::{Uuid, UuidValidator};
                        let pre_checked_id = if !b {a.as_id()?} else {a.to_string()};
                        self.#f_id = pre_checked_id.re_uuid();
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

//
/// Derive read-only [Mob] token stream.
//
fn generate_mob_impl(input: &DeriveInput) -> proc_macro2::TokenStream {
    let name = &input.ident;

    let Data::Struct(data) = &input.data else { panic!("Struct only!"); };

    let mwsz = maybe_field!(data, "max_weapon_size");
    let entsz = maybe_field!(data, "size");
    
    if mwsz.is_some() && entsz.is_some() {
        let max_weapon_size = mwsz.unwrap();
        let ent_size = entsz.unwrap();
        quote! {
            impl crate::mob::traits::Mob for #name {
                fn max_weapon_size(&self) -> crate::item::weapon::WeaponSize { self.#max_weapon_size }
                fn size(&self) -> crate::mob::core::EntitySize { self.#ent_size }
            }
        }
    } else {
        quote! {
            impl crate::mob::traits::Mob for #name {
                fn max_weapon_size(&self) -> crate::item::weapon::WeaponSize { crate::item::weapon::WeaponSize::Large }
                fn size(&self) -> crate::mob::core::EntitySize { crate::mob::core::EntitySize::Medium }
            }
        }
    }
}

//
/// Derive read-only [Mob].
//
#[proc_macro_derive(Mob)]
pub fn mob_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    TokenStream::from(generate_mob_impl(&input))
}

//
/// Derive read-only [Mob] and mutable [MobMut] both at once.
//
#[proc_macro_derive(MobMut)]
pub fn mob_mut_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

//    let base_impl = generate_mob_impl(&input);
    let Data::Struct(data) = input.data else { panic!("Struct only!") };
    let wsz_field = req_field!(data, "max_weapon_size");
    let sz_field = req_field!(data, "size");
    let mut_impl = quote! {
        impl crate::mob::traits::MobMut for #name {
            fn max_weapon_size_mut(&mut self) -> &mut crate::item::weapon::WeaponSize {
                &mut self.#wsz_field
            }
            fn size_mut(&mut self) -> &mut crate::mob::core::EntitySize {
                &mut self.#sz_field
            }
        }
    };
    TokenStream::from(quote! {
//        #base_impl
        #mut_impl
    })
}

/// Generate r/o [Itemized]'s token stream.
fn generate_itemized_impl(input: &DeriveInput) -> proc_macro2::TokenStream {
    let name = &input.ident;
    match &input.data {
        Data::Enum(data) => {
            let sizes = gen_container_match(&data, &format_ident!("size"), 0);
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
            let set_sizes = gen_container_match(&data, &format_ident!("set_size"), 1);

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

//
/// [Storage] related derive.
//
#[proc_macro_derive(Storage)]
pub fn storage_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    if let Data::Enum(data) = input.data {
        let can_holds = gen_container_match(&data, &format_ident!("can_hold"), 1);
        let max_spaces = gen_container_match(&data, &format_ident!("max_space"), 0);
        let req_spaces = gen_container_match(&data, &format_ident!("required_space"), 0);
        let spaces = gen_container_match(&data, &format_ident!("space"), 0);
        let try_inserts = gen_container_match(&data, &format_ident!("try_insert"), 1);
        let contains = gen_container_match(&data, &format_ident!("contains"), 1);
        let peek_ats = gen_container_match(&data, &format_ident!("peek_at"), 1);
        let peek_at_muts = gen_container_match(&data, &format_ident!("peek_at_mut"), 1);
        let takes = gen_container_match(&data, &format_ident!("take"), 1);
        let take_bys = gen_container_match(&data, &format_ident!("take_by_name"), 1);
        let find_id_by_names = gen_container_match(&data, &format_ident!("find_id_by_name"), 1);
        let ejects = gen_container_match(&data, &format_ident!("eject_all"), 0);

        TokenStream::from(quote! {
            impl crate::item::container::Storage for #name {
                fn can_hold(&self, a: &crate::item::Item) -> Result<(), crate::item::StorageQueryError> { match self {#(#can_holds),*}}
                fn max_space(&self) -> crate::item::StorageSpace { match self {#(#max_spaces),*}}
                fn required_space(&self) -> crate::item::StorageSpace { match self {#(#req_spaces),*}}
                fn space(&self) -> crate::item::StorageSpace { match self {#(#spaces),*}}
                fn try_insert(&mut self, a: crate::item::Item) -> Result<(), crate::item::StorageError> { match self {#(#try_inserts),*}}
                fn contains(&self, a: &str) -> bool { match self {#(#contains),*}}
                fn peek_at(&self, a: &str) -> Option<&Item> { match self {#(#peek_ats),*}}
                fn peek_at_mut(&mut self, a: &str) -> Option<&mut Item> { match self {#(#peek_at_muts),*}}
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

//
/// Generate [Describable] impl.
// 
fn generate_describable_impl(input: &DeriveInput) -> proc_macro2::TokenStream {
    let name = &input.ident;
    match &input.data {
        Data::Enum(data) => {
            let descs = gen_container_match(&data, &format_ident!("desc"), 0);
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

//
/// Derive [Describable].
//
#[proc_macro_derive(Describable, attributes(description))]
pub fn describable_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    TokenStream::from(generate_describable_impl(&input))
}

//
/// Derive [DescribableMut] (and [Describable]).
//
#[proc_macro_derive(DescribableMut, attributes(description))]
pub fn describable_mut_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let base_impl = generate_describable_impl(&input);
    let mut_impl = match &input.data
    {
        Data::Enum(data) => {
            let set_descs = gen_container_match(&data, &format_ident!("set_desc"), 1);
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
                        self.#f_desc = a.to_string();
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

//
/// Generate read-only [Owned] trait internals.
//
fn generate_owned_impl(input: &DeriveInput) -> proc_macro2::TokenStream {
    let name = &input.ident;

    match &input.data {
        Data::Enum(data) => {
            let owner_ids = gen_container_match(&data, &format_ident!("owner"), 0);
            let last_user_ids = gen_container_match(&data, &format_ident!("last_users"), 0);
            let sources = gen_container_match(&data, &format_ident!("source"), 0);
            
            quote! {
                impl crate::item::ownership::Owned for #name {
                    fn owner(&self) -> Option<String> { match self {#(#owner_ids),*}}
                    fn last_users(&self) -> Option<&std::collections::VecDeque<String>> { match self {#(#last_user_ids),*}}
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
                quote! { self.owner.last_users() },
                quote! { self.owner.source() },
            )
            } else {
            (
                quote! { self.owner_id.clone() },
                quote! { self.last_user_id.as_ref() },
                quote! { self.source.clone() },
            )
            };

            quote! {
                impl crate::item::ownership::Owned for #name {
                    fn owner(&self) -> Option<String> { #o_body }
                    fn last_users(&self) -> Option<&std::collections::VecDeque<String>> { #l_body }
                    fn source(&self) -> crate::item::ownership::ItemSource { #s_body }
                }
            }
        },
    
        _ => panic!("Only for Enum/Struct!")
    }
}

//
/// Derive [Owned].
//
#[proc_macro_derive(Owned)]
pub fn owned_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    TokenStream::from(generate_owned_impl(&input))
}

//
/// Generate [OwnedMut] trait internals.
//
fn generate_ownedmut_impl(input: &DeriveInput) -> proc_macro2::TokenStream {
    let name = &input.ident;

    match &input.data {
        Data::Enum(data) => {
            let set_owner_ids = gen_container_match(&data, &format_ident!("change_owner"), 1);
            let set_last_user_ids = gen_container_match(&data, &format_ident!("set_last_user"), 1);
            let set_sources = gen_container_match(&data, &format_ident!("set_source"), 3);

            let e_owner_ids = gen_container_match(&data, &format_ident!("erase_owner_r"), 0);
            let e_last_user_ids = gen_container_match(&data, &format_ident!("erase_last_user_r"), 0);
            let u_sources = gen_container_match(&data, &format_ident!("unify_source_r"), 3);
            quote! {
                impl crate::item::ownership::OwnedMut for #name {
                    fn change_owner(&mut self, a: &str) { match self {#(#set_owner_ids),*}}
                    fn set_last_user(&mut self, a: &str) -> Result<(), crate::identity::IdError> { match self {#(#set_last_user_ids),*}}
                    fn set_source(&mut self, a: &str, b: &str, c: &crate::item::ownership::ItemSource) -> Result<(), crate::item::ownership::ItemSourceError> { match self {#(#set_sources),*}}

                    fn erase_owner_r(&mut self) { match self {#(#e_owner_ids),*}}
                    fn erase_last_user_r(&mut self) { match self {#(#e_last_user_ids),*}}
                    fn unify_source_r(&mut self, a: &str, b: &str, c: &crate::item::ownership::ItemSource) -> Result<(), crate::item::ownership::ItemSourceError> { match self {#(#u_sources),*}}
                }
            }
        }
        
        Data::Struct(data) => {
            let has_owner_field = data.fields.iter().any(|f| {
                f.ident.as_ref().map(|i| i == "owner").unwrap_or(false)
            });
            let has_contents_field = data.fields.iter().any(|f|{
                f.ident.as_ref().map(|i| i == "contents").unwrap_or(false)
            });

            let (o_body, l_body, s_body,
                e_o_body, e_l_body, u_s_body
                ) = if has_owner_field {(
                quote! { self.owner.change_owner(a) },
                quote! { self.owner.set_last_user(a) },
                quote! { self.owner.set_source(a,b,c) },
                quote! { self.owner.erase_owner_r() },
                quote! { self.owner.erase_last_user_r() },
                quote! { self.owner.unify_source_r(a,b,c) },
            )} else {(
                // change_owner
                quote! {
                    if let Some(ref mut prev_owner) = self.owner_id {
                        log::trace!("Changing ownership from '{}' to '{}'", prev_owner, a);
                        *prev_owner = a.to_string();
                    } else {
                        self.owner_id = a.to_string().into();
                    }
                },

                // set_last_user
                quote! {
                    crate::identity::uniq::is_id(a)?;
                    if let Some(luid) = &mut self.last_user_id {
                        luid.push_front(a.to_string());
                    } else {
                        let mut v = std::collections::VecDeque::new();
                        v.push_back(a.to_string());
                        self.last_user_id = Some(v);
                    }
                    Ok(())
                },

                // set_source
                quote! {
                    if let crate::item::ownership::ItemSource::Blueprint = c {
                        log::warn!("Hol'up! Rejecting demotion of '{}' to blueprint by '{}'.", a, b);
                        return Err(crate::item::ownership::ItemSourceError::Rejected);
                    }
                    self.source = c.clone();
                    Ok(())
                },

                // erase owner
                quote! { self.owner_id = None; },

                // erase last users
                quote! { self.last_user_id = None; },

                // unify sources
                quote! { self.set_source(a,b,c) },
            )};

            if has_contents_field {
                quote! {
                    impl crate::item::ownership::OwnedMut for #name {
                        fn change_owner(&mut self, a: &str) { #o_body }
                        fn set_last_user(&mut self, a: &str) -> Result<(), crate::identity::IdError> { #l_body }
                        fn set_source(&mut self, a: &str, b: &str, c: &crate::item::ownership::ItemSource) -> Result<(), crate::item::ownership::ItemSourceError> { #s_body }

                        fn erase_owner_r(&mut self) {
                            #e_o_body;
                            for (_,i) in self.contents.iter_mut() {
                                i.erase_owner_r();
                            }
                        }
                        fn erase_last_user_r(&mut self) {
                            #e_l_body;
                            for (_,i) in self.contents.iter_mut() {
                                i.erase_last_user_r();
                            }
                        }
                        fn unify_source_r(&mut self, a: &str, b: &str, c: &crate::item::ownership::ItemSource) -> Result<(), crate::item::ownership::ItemSourceError> {
                            let ok = #u_s_body;
                            // it's fine to recurse, no ItemSourceError
                            if let Ok(ok) = ok {
                                for (_,i) in self.contents.iter_mut() {
                                    i.unify_source_r(a,b,c).ok();
                                }
                            }
                            ok
                        }
                    }
                }
            } else {
                quote! {
                    impl crate::item::ownership::OwnedMut for #name {
                        fn change_owner(&mut self, a: &str) { #o_body }
                        fn set_last_user(&mut self, a: &str) -> Result<(), crate::identity::IdError> { #l_body }
                        fn set_source(&mut self, a: &str, b: &str, c: &crate::item::ownership::ItemSource) -> Result<(), crate::item::ownership::ItemSourceError> { #s_body }

                        fn erase_owner_r(&mut self) { #e_o_body }
                        fn erase_last_user_r(&mut self) { #e_l_body }
                        fn unify_source_r(&mut self, a: &str, b: &str, c: &crate::item::ownership::ItemSource) -> Result<(), crate::item::ownership::ItemSourceError> { #u_s_body }
                    }
                }
            }
        }

        _ => unimplemented!("Only for Enum/Struct!")
    }
}

//
/// Derive [OwnedMut] and [Owned].
//
#[proc_macro_derive(OwnedMut)]
pub fn owned_mut_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let base_impl = generate_owned_impl(&input);
    let mut_impl = generate_ownedmut_impl(&input);

    TokenStream::from(quote! {
        #base_impl
        #mut_impl
    })
}

//
/// Generate read-only [Combatant] token stream.
//
fn generate_combatant_impl(input: &DeriveInput) -> proc_macro2::TokenStream {
    let name = &input.ident;
    let Data::Struct(data) = &input.data else { panic!("Struct only!"); };
    let hp_field = req_field!(data, "hp");
    let mp_field = req_field!(data, "mp");
    let sn_field = req_field!(data, "sn");
    let san_field = req_field!(data, "san");
    let brn_field = req_field!(data, "brn");
    let str_field = req_field!(data, "strn");
    let nim_field = req_field!(data, "nim");
    let loc_field = req_field!(data, "location");

    quote! {
        impl crate::combat::Combatant for #name {
            fn hp(&self) -> &crate::mob::stat::Stat { &self.#hp_field }
            fn mp(&self) -> &crate::mob::stat::Stat { &self.#mp_field }
            fn sn(&self) -> &crate::mob::stat::Stat { &self.#sn_field }
            fn san(&self) -> &crate::mob::stat::Stat { &self.#san_field }
            fn nim(&self) -> &crate::mob::stat::Stat { &self.#nim_field }
            fn brn(&self) -> &crate::mob::stat::Stat { &self.#brn_field }
            fn str(&self) -> &crate::mob::stat::Stat { &self.#str_field }
            fn location(&self) -> std::sync::Weak<tokio::sync::RwLock<crate::room::Room>> { self.#loc_field.clone() }
        }
    }
}

//
/// Derive read-only [Combatant].
//
#[proc_macro_derive(Combatant)]
pub fn combatant_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    TokenStream::from(generate_combatant_impl(&input))
}

/// Derive read-only [Combatant] and mutable [Mut] both at once.
#[proc_macro_derive(CombatantMut)]
pub fn combatant_mut_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let base_impl = generate_combatant_impl(&input);
    let Data::Struct(data) = input.data else { panic!("Struct only!") };

    let hp_field = req_field!(data, "hp");
    let mp_field = req_field!(data, "mp");
    let sn_field = req_field!(data, "sn");
    let san_field = req_field!(data, "san");
    let brn_field = req_field!(data, "brn");
    let nim_field = req_field!(data, "nim");
    let str_field = req_field!(data, "strn");
    let loc_field = req_field!(data, "location");
    let inv_field = req_field!(data, "inventory");
    let freeza_field = maybe_field!(data, "brain_freeze");
    let brain_freeze_logic = if let Some(field) = freeza_field {
        quote! { self.#field = freeze; }
    } else {
        quote! {}
    };
    
    let mut_impl = quote! {
        impl crate::combat::CombatantMut for #name {
            fn hp_mut(&mut self) -> &mut crate::mob::stat::Stat { &mut self.#hp_field }
            fn mp_mut(&mut self) -> &mut crate::mob::stat::Stat { &mut self.#mp_field }
            fn sn_mut(&mut self) -> &mut crate::mob::stat::Stat { &mut self.#sn_field }
            fn san_mut(&mut self) -> &mut crate::mob::stat::Stat { &mut self.#san_field }
            fn brn_mut(&mut self) -> &mut crate::mob::stat::Stat { &mut self.#brn_field }
            fn nim_mut(&mut self) -> &mut crate::mob::stat::Stat { &mut self.#nim_field }
            fn str_mut(&mut self) -> &mut crate::mob::stat::Stat { &mut self.#str_field }
            fn take_dmg(&mut self, dmg: crate::mob::StatValue) -> bool {
                *(self.hp_mut()) -= dmg.abs();// no "healing" with dmg…
                self.is_dead()
            }

            fn heal(&mut self, dmg: crate::mob::StatValue) {
                *(self.hp_mut()) += dmg.abs();// no "dmg" with healing…
            }

            fn inventory(&mut self) -> &mut crate::item::container::variants::ContainerVariant {
                &mut self.#inv_field
            }

            fn set_location(&mut self, arc: &std::sync::Arc<tokio::sync::RwLock<crate::room::Room>>) {
                self.#loc_field = std::sync::Arc::downgrade(arc);
            }

            fn alter_brain_freeze(&mut self, freeze: bool) {
                #brain_freeze_logic
            }
        }
    };
    TokenStream::from(quote! {
        #base_impl
        #mut_impl
    })
}

//
/// Generate read-only [Factioned] token stream.
//
fn generate_factioned_impl(input: &DeriveInput) -> proc_macro2::TokenStream {
    let name = &input.ident;

    let Data::Struct(data) = &input.data else { panic!("Struct only!"); };
    let fact_field = req_field!(data, "faction");

    quote! {
        impl crate::mob::faction::Factioned for #name {
            fn faction(&self) -> crate::mob::faction::EntityFaction { self.#fact_field.clone() }
        }
    }
}

//
/// Derive read-only [Factioned].
//
#[proc_macro_derive(Factioned)]
pub fn factioned_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    TokenStream::from(generate_factioned_impl(&input))
}

//
/// Derive read-only [Factioned] and mutable [FactionMut] both at once.
//
#[proc_macro_derive(FactionMut)]
pub fn faction_mut_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let base_impl = generate_factioned_impl(&input);
    let Data::Struct(data) = input.data else { panic!("Struct only!") };
    let fact_field = req_field!(data, "faction");
    let mut_impl = quote! {
        impl crate::mob::faction::FactionMut for #name {
            fn faction_mut(&mut self) -> &mut crate::mob::faction::EntityFaction {
                &mut self.#fact_field
            }
        }
    };
    TokenStream::from(quote! {
        #base_impl
        #mut_impl
    })
}
