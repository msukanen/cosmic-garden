//! Corpse spec.

use std::{collections::{HashMap, VecDeque}, sync::Arc};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::{cmd::CommandCtx, err_tell_user, identity::{IdError, IdentityMut, IdentityQuery, uniq::StrUuid}, item::{Item, container::{ContainerSpec, Storage, StorageSpace, storage::{StorageError, StorageQueryError}, variants::ContainerVariant}, ownership::{ItemSource, ItemSourceError, Owned, OwnedMut}}, mob::core::Entity, player::Player, room::Room, tell_user, thread::add_item_to_lnf};

mod arc_n_ent_transform {
    use std::sync::Arc;
    use serde::{Deserialize, Deserializer, Serializer};
    use tokio::sync::RwLock;
    use crate::mob::core::Entity;

    pub fn serialize<S>(what: &Option<Arc<RwLock<Entity>>>, s:S) -> Result<S::Ok, S::Error>
    where S: Serializer
    {
        match what {
            Some(what) => {
                if let Ok(guard) = what.try_read() {
                    // try_read - skip if contested right now
                    s.serialize_some(&*guard)
                } else { s.serialize_none() }
            }
            _ => s.serialize_none()
        }
    }

    pub fn deserialize<'de, D>(d: D) -> Result< Option<Arc<RwLock<Entity>> >, D::Error>
    where D: Deserializer<'de>
    {
        let opt: Option<Entity> = Option::deserialize(d)?;
        Ok(opt.map(|ent| Arc::new(RwLock::new(ent))))
    }
}

/// Corpse spec.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CorpseSpec {
    pub(crate) spec: ContainerSpec,
    #[serde(default, with = "arc_n_ent_transform")]
    pub(crate) possessed_by: Option<Arc<RwLock<Entity>>>,
}

impl IdentityMut for CorpseSpec {
    fn set_id(&mut self, value: &str, m_id: bool) -> Result<(), IdError> {
        self.spec.set_id(value, m_id)
    }

    fn set_title(&mut self, value: &str) {
        self.spec.set_title(value);
    }

    fn title_mut<'a>(&'a mut self) -> &'a mut String {
        self.spec.title_mut()
    }
}

impl IdentityQuery for CorpseSpec {
    fn id<'a>(&'a self) -> &'a str {
        self.spec.id()
    }

    fn title<'a>(&'a self) -> &'a str {
        self.spec.title()
    }
}

impl Owned for CorpseSpec {
    fn last_users(&self) -> Option<&VecDeque<String>> { None }
    fn owner(&self) -> Option<String> { None }
    fn source(&self) -> ItemSource { ItemSource::System }
}

impl OwnedMut for CorpseSpec {
    fn change_owner(&mut self, _: &str) {}
    fn set_last_user(&mut self, _: &str) -> Result<(), IdError> { Ok(()) }
    fn set_source(&mut self, _: &str, _: &str, _: &ItemSource) -> Result<(), ItemSourceError> { Ok(()) }
    fn erase_owner_r(&mut self) {}
    fn erase_last_user_r(&mut self) {}
    fn unify_source_r(&mut self, _: &str, _: &str, _: &ItemSource) -> Result<(), ItemSourceError> { Ok(()) }
}

impl Storage for CorpseSpec {
    /// Corpses reject items and thus `can_hold()` will
    /// [**auto-error**][StorageQueryError::NotContainer].
    fn can_hold(&self, _: &Item) -> Result<(), StorageQueryError> {
        //self.spec.can_hold(item)
        Err(StorageQueryError::NotContainer)
    }
    fn contains(&self, id: &str) -> bool { self.spec.contains(id) }
    fn eject_all(&mut self) -> Option<Vec<Item>> { self.spec.eject_all() }
    fn find_id_by_name(&self, name: &str) -> Option<String> { self.spec.find_id_by_name(name) }
    /// Corpses reject items…
    fn max_space(&self) -> StorageSpace {
        //self.spec.max_space()
        0
    }
    fn peek_at(&self, id: &str) -> Option<&Item> { self.spec.peek_at(id) }
    fn peek_at_mut(&mut self, id: &str) -> Option<&mut Item> { self.spec.peek_at_mut(id) }
    fn required_space(&self) -> StorageSpace { self.spec.required_space() }
    /// Corpses reject items…
    fn space(&self) -> StorageSpace {
        //self.spec.space()
        0
    }
    fn take(&mut self, id: &str) -> Option<Item> { self.spec.take(id) }
    fn take_by_name(&mut self, id: &str) -> Option<Item> { self.spec.take_by_name(id) }
    /// Corpses reject items and thus `try_insert()` will
    /// [**auto-error**][StorageError::NotContainer].
    fn try_insert(&mut self, item: Item) -> Result<(), StorageError> {
        // self.spec.try_insert(item)
        Err(StorageError::NotContainer(item))
    }
}

impl CorpseSpec {
    // file-private .try_insert() bypass.
    fn bypass_try_insert(&mut self, item: Item) {
        self.spec.contents.insert(item.id().to_string(), item);
    }
}

/// Bulk transfer everything within `from` to `plr` inventory who is at `room`.
pub async fn bulk_transfer(ctx: &mut CommandCtx<'_>, plr: Arc<RwLock<Player>>, room: Arc<RwLock<Room>>, from: &str) {
    if let Some(src) = room.write().await.peek_at_mut(from) {
        if !matches!(*src, Item::Container(_)|Item::Corpse{..}) {
            err_tell_user!(ctx.writer, "'{}' doesn't contain anything. Did you mean to pick it insted?\n", from.show_uuid(false))
        }
        let Some(items) = src.eject_all() else {
            err_tell_user!(ctx.writer, "Bummer, '{}' seems to be empty!\n", from.show_uuid(false))
        };
        let mut not_taken = vec![];
        let mut taken_names = vec![];
        // Shove the things into player's inventory, if we can.
        {
            let mut p = plr.write().await;
            for item in items {
                let name = item.title().to_string();
                if let Err(e) = p.inventory.try_insert(item) {
                    not_taken.push(e.extract_item());
                    continue;
                }
                taken_names.push(name);
            }
        }
        let ntl = not_taken.len();
        for item in not_taken {
            // as we can't use .try_insert() with corpses, we try something else...
            if let Item::Corpse{loot: ContainerVariant::Corpse(spec),..} = src {
                spec.bypass_try_insert(item);
            } else if let Err(e) = src.try_insert(item) {
                log::error!("Something fishy with {src:?} - cannot insert - {e:?}");
                add_item_to_lnf(e).await;
            }
        }
        
        if taken_names.is_empty() {
            let (a,s,ait) = match ntl {
                1 => ("a ","",""),
                _ => ("", "s", "any of ")
            };
            err_tell_user!(ctx.writer, "There's {}thing{}, but you don't seem to be able to carry {}it…\n", a,s,ait);
        } else {
            let mut things: HashMap<String, u32> = HashMap::new();
            let total = taken_names.len();
            for n in taken_names {
                things.entry(n).and_modify(|c| *c += 1).or_insert(1);
            }
            if things.keys().len() < 6 {
                tell_user!(ctx.writer, "You nab: {}\n", things.into_iter().map(|(n,x)| {
                    let (num, s) = match x {
                        0 => ("none of".into(), "s"),
                        1 => ("one".into(), ""),
                        x => (format!("{x}"), "s")
                    };
                    format!("{num} {n}{s}")
                }).collect::<Vec<String>>().join(", "));
            } else {
                tell_user!(ctx.writer, "You nab a bunch of things ({} in total).\n", total);
            }
        }
    }
}
