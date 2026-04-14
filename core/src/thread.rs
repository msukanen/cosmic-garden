//! Nexus of threading…
pub(crate) mod janitor; pub(crate) use janitor as io;
pub(crate) mod librarian; pub(crate) use librarian as lib;
pub(crate) mod life_thread; pub(crate) use life_thread as game;
