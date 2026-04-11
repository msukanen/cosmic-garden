//! Command aliasing.

use std::{collections::HashMap};

use lazy_static::lazy_static;

use crate::io::DATA_PATH;

lazy_static! {
    /// Command aliasing lives here…
    pub(crate) static ref CMD_ALIASES: HashMap<String, String> = {
        let contents = std::fs::read_to_string(&format!("{}/cmd_alias.json", *DATA_PATH)).unwrap_or_default();
        serde_json::from_str(&contents).unwrap_or_default()
    };
}

#[cfg(test)]
mod cmd_alias_tests {
    use std::env;

    use crate::{DATA, cmd::cmd_alias::CMD_ALIASES};

    #[test]
    fn cmd_alias_reads() {
        let _ = DATA.set(env::var("COSMIC_GARDEN_DATA").unwrap());
        let _ = (*CMD_ALIASES).clone();
        assert_eq!("inventory".to_string(), *CMD_ALIASES.get("inv").unwrap());
    }
}
