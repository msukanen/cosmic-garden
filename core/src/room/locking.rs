//! Entry locking mechanisms and such.

use std::sync::{Arc, Weak};

use tokio::sync::RwLock;

use crate::{identity::{IdentityQuery, uniq::StrUuid}, item::Item, room::Room};

#[derive(Debug, PartialEq)]
pub enum LockingError {
    NoLock,
    NotKey,
    WrongKey,
    Free,
    AlreadyOpen,
    AlreadyClosed,  
    AlreadyLocked,  Locked, NotLocked,
    RoomNotFound,
}

/// [Room] exit variants.
pub enum Exit {
    /// No distinct "door" or alike to deal with.
    Free { room: Weak<RwLock<Room>> },
    /// An open entrance which may or may not have a lock.
    Open { key_bp: Option<String>, room: Weak<RwLock<Room>> },
    /// An closed entrance which may or may not have a lock.
    Closed { key_bp: Option<String>, room: Weak<RwLock<Room>> },
    /// A locked entrance.
    Locked { key_bp: String, room: Weak<RwLock<Room>> },
    /// An open, autolocking entrance.
    OpenAL { key_bp: String, room: Weak<RwLock<Room>> },
    /// A locked autolocking entrance.
    LockedAL { key_bp: String, room: Weak<RwLock<Room>> },
}

impl Exit {
    /// Get the destination `Weak` [Room].
    pub fn as_weak(&self) -> Weak<RwLock<Room>> {
        match self {
            Self::Free { room }        |
            Self::Closed { room,.. }   |
            Self::Locked { room,.. }   |
            Self::Open { room,.. }     |
            Self::LockedAL { room,.. } |
            Self::OpenAL { room,.. }   => room.clone()
        }
    }

    /// Try upgrade destination `Weak` [Room] to `Arc`.
    pub fn as_arc(&self) -> Option<Arc<RwLock<Room>>> {
        match self {
            Self::Free { room }        |
            Self::Closed { room,.. }   |
            Self::Locked { room,.. }   |
            Self::LockedAL { room,.. } |
            Self::OpenAL { room,.. }   |
            Self::Open { room,.. }     => room.upgrade()
        }
    }

    /// Connect the [Exit] to the given [Room].
    pub fn reroute(&mut self, dest: Arc<RwLock<Room>>) {
        match self {
            Self::Free { room }        |
            Self::Closed { room,.. }   |
            Self::Locked { room,.. }   |
            Self::LockedAL { room,.. } |
            Self::Open { room,.. }     |
            Self::OpenAL { room,.. }   => *room = Arc::downgrade(&dest)
        }
    }

    /// Try lock the [Exit].
    pub fn try_lock(&mut self, key: &Item) -> Result<(), LockingError> {
        match &self {
            Self::Free { .. }               |
            Self::Open { key_bp: None,..}   |
            Self::Closed { key_bp: None,..} => Err(LockingError::NoLock),

            Self::LockedAL { .. } |
            Self::Locked { .. }   => Err(LockingError::AlreadyLocked),
            
            Self::Open { key_bp: Some(id), room } |
            Self::Closed { key_bp: Some(id), room } => {
                if id == key.id().show_uuid(false) {
                    *self = if matches!(self, Self::Open {..}|Self::Closed {..}) {
                        Self::Locked { key_bp: id.clone(), room: room.clone() }
                    } else {
                        Self::LockedAL { key_bp: id.clone(), room: room.clone() }
                    };
                    Ok(())
                } else {
                    Err(LockingError::WrongKey)
                }
            }
            Self::OpenAL { key_bp, room } => {
                *self = Self::LockedAL { key_bp: key_bp.clone(), room: room.clone() };
                Ok(())
            }
        }
    }

    /// Try open the [Exit].
    /// 
    /// Opening a [free exit][Exit::Free] exit isn't a hard error,
    /// but will be notified as optional payload for `Ok()`.
    pub fn try_open(&mut self, key: Option<&Item>, unlock_only: bool) -> Result<Option<LockingError>, LockingError> {
        match self {
            Self::Free { .. }   => Ok(Some(LockingError::Free)),
            Self::Open { .. }   |
            Self::OpenAL { .. } => Err(LockingError::AlreadyOpen),
            Self::Closed { key_bp, room } => {
                *self = Self::Open { key_bp: key_bp.clone(), room: room.clone() };
                Ok(None)
            },
            Self::Locked { key_bp, room } => {
                let Some(key) = key else { return Err(LockingError::NotKey) };
                if key_bp == key.id().show_uuid(false) {
                    *self = if unlock_only {
                        Self::Closed { key_bp: Some(key_bp.clone()), room: room.clone() }
                    } else {
                        Self::Open { key_bp: Some(key_bp.clone()), room: room.clone() }
                    };
                    Ok(None)
                } else {
                    Err(LockingError::WrongKey)
                }
            }
            Self::LockedAL { key_bp, room } => {
                let Some(key) = key else { return Err(LockingError::NotKey) };
                if key_bp == key.id().show_uuid(false) {
                    *self = Self::OpenAL { key_bp: key_bp.clone(), room: room.clone() };
                    Ok(None)
                } else {
                    Err(LockingError::WrongKey)
                }
            }
        }
    }

    /// Try close the [Exit].
    pub fn try_close(&mut self) -> Result<(), LockingError> {
        match self {
            Self::Free { .. } => Err(LockingError::Free),
            Self::Open { key_bp, room} => {
                *self = Self::Closed { key_bp: key_bp.clone(), room: room.clone() };
                Ok(())
            },
            // autolock OpenAL if closed.
            Self::OpenAL { key_bp, room } => {
                *self = Self::LockedAL { key_bp: key_bp.clone(), room: room.clone() };
                Ok(())
            }
            Self::Closed { .. }   |
            Self::Locked { .. }   |
            Self::LockedAL { .. } => Err(LockingError::AlreadyClosed)
        }
    }

    /// Attempt to change the [Exit]'s key.
    pub fn rekey(&mut self, key_id: String) -> Result<(), LockingError> {
        match self {
            Self::Free { .. } => Err(LockingError::NoLock),
            Self::Open { key_bp,.. }   |
            Self::Closed { key_bp,.. } => {
                *key_bp = Some(key_id);
                Ok(())
            }
            Self::Locked { key_bp,.. } |
            Self::OpenAL { key_bp,.. } |
            Self::LockedAL { key_bp,.. } => {
                *key_bp = key_id;
                Ok(())
            }
        }
    }

    /// Remove locking entirely.
    /// 
    /// It's not a hard error to try remove lock from [free exit][Exit::Free] exit,
    /// but it'll be notified about as an optional [LockingError].
    pub fn remove_lock(&mut self) -> Option<LockingError> {
        match self {
            Self::Free { .. } => Some(LockingError::Free),
            Self::Closed { key_bp: None,.. } |
            Self::Open { key_bp: None,.. }   => Some(LockingError::NoLock),
            Self::Closed { key_bp,.. } |
            Self::Open { key_bp,.. }   => { *key_bp = None; None },
            Self::Locked { room,.. }   |
            Self::LockedAL { room,.. } => {
                *self = Self::Closed { key_bp: None, room: room.clone() };
                None
            },
            Self::OpenAL { room, .. } => {
                *self = Self::Open { key_bp: None, room: room.clone() };
                None
            }
        }
    }
}

#[cfg(test)]
mod exit_locking_tests {
    use std::sync::Arc;

    use crate::{item::{Item, TemporaryStructToAppeaseAnalyzerDuringWIP, ownership::Owner}, room::{Room, locking::{Exit, LockingError}}};

    fn init() {
        let _ = crate::DATA.get_or_init(|| "data".into());
        let _ = crate::WORLD.get_or_init(|| "crash-test-dummy".to_string());
    }

    #[tokio::test]
    async fn exit_close() {
        init();
        let r1 = Room::new("r-1", "Room#1").await.unwrap();
        let mut ex1 = Exit::Open { key_bp: Some("key-1".into()), room: Arc::downgrade(&r1) };
        if let Err(e) = ex1.try_close() {
            panic!("LockingError: {e:?}");
        }
        assert!(matches!(ex1, Exit::Closed { .. }));
    }

    #[tokio::test]
    async fn exit_open() {
        init();
        let r1 = Room::new("r-1", "Room#1").await.unwrap();
        let mut ex1 = Exit::Closed { key_bp: Some("key-1".into()), room: Arc::downgrade(&r1) };
        if let Err(e) = ex1.try_open(None, false) {
            panic!("LockingError: {e:?}");
        }
        assert!(matches!(ex1, Exit::Open { .. }));
        assert_eq!(Err(LockingError::AlreadyOpen), ex1.try_open(None, false));
        let mut ex1 = Exit::Free { room: Arc::downgrade(&r1) };
        assert_eq!(Ok(Some(LockingError::Free)), ex1.try_open(None, false));
    }

    #[tokio::test]
    async fn exit_locked_open() {
        init();
        let r1 = Room::new("r-1", "Room#1").await.unwrap();
        let k1 = Item::Key(TemporaryStructToAppeaseAnalyzerDuringWIP { id: "key-1".into(), title: "Key#1".into(), owner: Owner::no_one() });
        let k2 = Item::Key(TemporaryStructToAppeaseAnalyzerDuringWIP { id: "key-2".into(), title: "Key#2".into(), owner: Owner::no_one() });
        let mut ex1 = Exit::Locked { key_bp: "key-1".into(), room: Arc::downgrade(&r1) };
        assert_eq!(Err(LockingError::NotKey), ex1.try_open(None, false));
        assert_eq!(Err(LockingError::WrongKey), ex1.try_open(Some(&k2), false));
        assert!(match ex1.try_open(Some(&k1), false) {
            Ok(_) => true,
            _ => false,
        });
    }

    #[tokio::test]
    async fn exit_lock() {
        init();
        let r1 = Room::new("r-1", "Room#1").await.unwrap();
        let k1 = Item::Key(TemporaryStructToAppeaseAnalyzerDuringWIP { id: "key-1".into(), title: "Key#1".into(), owner: Owner::no_one() });
        let k2 = Item::Key(TemporaryStructToAppeaseAnalyzerDuringWIP { id: "key-2".into(), title: "Key#2".into(), owner: Owner::no_one() });
        let mut ex1 = Exit::Free { room: Arc::downgrade(&r1) };
        assert_eq!(Err(LockingError::NoLock), ex1.try_lock(&k1));
        let mut ex1 = Exit::Open { key_bp: None, room: Arc::downgrade(&r1) };
        assert_eq!(Err(LockingError::NoLock), ex1.try_lock(&k1));
        let mut ex1 = Exit::Open { key_bp: Some("key-1".into()), room: Arc::downgrade(&r1) };
        assert_eq!(Err(LockingError::WrongKey), ex1.try_lock(&k2));
        assert_eq!(Ok(()), ex1.try_lock(&k1));
        assert_eq!(Err(LockingError::AlreadyLocked), ex1.try_lock(&k2));
    }
}
