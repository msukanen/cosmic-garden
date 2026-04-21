fn escape_hatch(villain: &str) -> String {
    match villain {
        // --- strict keywords ---
        "as"       | "break"    | "const"    | "continue" | "crate"    | 
        "else"     | "enum"     | "extern"   | "false"    | "fn"       | 
        "for"      | "if"       | "impl"     | "in"       | "let"      | 
        "loop"     | "match"    | "mod"      | "move"     | "mut"      | 
        "pub"      | "ref"      | "return"   | "self"     | "static"   | 
        "struct"   | "super"    | "trait"    | "true"     | "type"     | 
        "unsafe"   | "use"      | "where"    | "while"    | "async"    | 
        "await"    | "dyn"      | "abstract" | "become"   | "box"      | 
        // --- reserved / future Keywords ---
        "do"       | "final"    | "macro"    | "override" | "priv"     | 
        "typeof"   | "unsized"  | "virtual"  | "yield"    | "try"      |
        // --- type level & maybe ---
        "Self"
        => format!("r#{}", villain),
        _ => villain.into(),
    }
}

pub(crate) const VILLAIN_ID: [&'static str; 51] = [
    "as"       , "break"    , "const"    , "continue" , "crate"    ,
    "else"     , "enum"     , "extern"   , "false"    , "fn"       , 
    "for"      , "if"       , "impl"     , "in"       , "let"      , 
    "loop"     , "match"    , "mod"      , "move"     , "mut"      , 
    "pub"      , "ref"      , "return"   , "self"     , "static"   , 
    "struct"   , "super"    , "trait"    , "true"     , "type"     , 
    "unsafe"   , "use"      , "where"    , "while"    , "async"    , 
    "await"    , "dyn"      , "abstract" , "become"   , "box"      , 
    // --- reserved / future Keywords ---
    "do"       , "final"    , "macro"    , "override" , "priv"     , 
    "typeof"   , "unsized"  , "virtual"  , "yield"    , "try"      ,
    // --- type level & maybe ---
    "Self"
];
