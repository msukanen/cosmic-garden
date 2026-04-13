//! Sluggery!

use unicode_normalization::UnicodeNormalization;

use crate::{identity::{IdError, MAX_ID_LEN}, string::Uuid};

/// A trait for all the slugs…
pub trait Slugger {
    /// Overwrite all non-alphabet with underscores.
    fn slugify(&self) -> String;
    /// Get representation as file/hash ready `id`, if possible…
    fn as_id(&self) -> Result<String, IdError>;
    /// Check if string is valid as ID without actually creating an ID out of it.
    fn is_id(&self) -> Result<(), IdError>;
    /// Reduce repetitive non-alphanum to singular entries.
    fn reduce_noise(&self) -> Result<String, IdError>;
}

impl Slugger for String {
    #[inline] fn as_id(&self) -> Result<String, IdError> { as_id(self) }
    #[inline] fn is_id(&self) -> Result<(), IdError> { is_id(self) }
    #[inline] fn reduce_noise(&self) -> Result<String, IdError> { reduce_noise(self) }
    #[inline] fn slugify(&self) -> String { slugify(self) }
}

impl Slugger for &String {
    #[inline] fn as_id(&self) -> Result<String, IdError> { as_id(self) }
    #[inline] fn is_id(&self) -> Result<(), IdError> { is_id(self) }
    #[inline] fn reduce_noise(&self) -> Result<String, IdError> { reduce_noise(self) }
    #[inline] fn slugify(&self) -> String { slugify(self) }
}

impl Slugger for &str {
    #[inline] fn as_id(&self) -> Result<String, IdError> { as_id(self) }
    #[inline] fn is_id(&self) -> Result<(), IdError> { is_id(self) }
    #[inline] fn reduce_noise(&self) -> Result<String, IdError> { reduce_noise(self) }
    #[inline] fn slugify(&self) -> String { slugify(self) }
}

/// Overwrite all non-alphabet with underscores.
/// 
/// # Args
/// - `input` which possibly requires overwrites.
fn slugify(input: &str) -> String {
    input.nfd()
        .map(|c| {
            if c.is_ascii_alphabetic() || c == '-'
                 { c }
            else {'_'}
        })
        .collect()
}

/// Get representation as file/hash ready `id`, if possible…
/// 
/// # Args
/// - `input` to be sanitized.
fn as_id(input: &str) -> Result<String, IdError> {
    let mut out = String::new();
    let mut last_was_junk = false;
    let mut has_alnum = false;

    for ch in input.trim().to_lowercase().nfd() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_was_junk = false;
            has_alnum = true;
            continue;
        }

        if !last_was_junk && !out.is_empty() {
            out.push(match ch {
                '-' => '-',
                _ if ch.is_whitespace() => '-',
                _ => '_'
            });
            last_was_junk = true;
        }
    }

    if !has_alnum || out.is_empty() {
        return Err(IdError::EmptyOrGarbage);
    }

    if out.len() > MAX_ID_LEN {
        return Err(IdError::TooLong);
    }

    Ok(out)
}

/// Check if string is valid as ID without actually creating an ID out of it.
pub fn is_id(input: &str) -> Result<(), IdError> {
    let mut out = 0;
    let mut last_was_junk = false;
    let mut has_alnum = false;

    for ch in input.trim().to_lowercase().nfd() {
        if ch.is_ascii_alphanumeric() {
            out += 1;
            last_was_junk = false;
            has_alnum = true;
            continue;
        }

        if !last_was_junk && out != 0 {
            out += 1;
            last_was_junk = true;
        }
    }

    if !has_alnum || out == 0 {
        return Err(IdError::EmptyOrGarbage);
    }

    if out > MAX_ID_LEN {
        return Err(IdError::TooLong);
    }

    Ok(())
}

/// Reduce repetitive non-alphanum to singular entries.
/// 
/// # Args
/// - `input` to be reduced.
fn reduce_noise(input: &str) -> Result<String, IdError> {
    let mut out = String::new();
    let mut last_was_junk = None;
    let mut has_alnum = false;

    for ch in input.trim().to_lowercase().nfd() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_was_junk = None;
            has_alnum = true;
            continue;
        }

        if let Some(junk) = last_was_junk {
            if junk != ch {
                out.push(ch);
                last_was_junk = Some(ch);
                continue;
            }
        } else {
            last_was_junk = Some(ch);
            out.push(ch);
        }
    }

    if !has_alnum || out.is_empty() {
        return Err(IdError::EmptyOrGarbage);
    }

    if out.len() > MAX_ID_LEN {
        return Err(IdError::TooLong);
    }

    Ok(out)
}

pub fn as_id_with_uuid(value: &str) -> Result<String, IdError> {
    let base_id = value.as_id()?;
    Ok(base_id.with_uuid())
}

#[cfg(test)]
mod slug_tests {
    use crate::string::UUID_RE;

    use super::*;


    #[test]
    fn as_id() {
        let src = "Ali  bab ---atsuu";
        if let Ok(out) = src.as_id() {
            assert_ne!(src, out.as_str());
            assert_eq!("ali-bab-atsuu", out.as_str());
        } else {
            panic!(".as_id() is broken! Fix!")
        }
    }

    #[test]
    fn as_id_uuid() {
        let src = "550E8400-E29b-41D4-A716-446655440000";
        let fst_out;
        if let Ok(out) = src.as_id() {
            fst_out = out;
            assert_ne!(src, fst_out.as_str());
            assert_eq!("550e8400-e29b-41d4-a716-446655440000", fst_out.as_str());
        } else {
            panic!(".as_id() is broken! Fix!")
        }

        let out = src.re_uuid();
        assert!(UUID_RE.is_match(&out));
        assert_ne!(fst_out, out);
    }

    #[test]
    fn slugify() {
        let src = "blob#!!#$$2";
        let out = src.slugify();
        assert_ne!("blob2", out.as_str());
        assert_eq!("blob_______", out.as_str());
    }

    #[test]
    fn reduce() {
        let _ = env_logger::try_init();
        let src = "blob#!!#$$2";
        let noiseless = super::reduce_noise(src);
        if let Ok(s) = noiseless {
            assert_eq!("blob#!#$2", s);
        } else { panic!("Oh noes… got: {noiseless:?}")};
        
        let src = src.with_uuid();
        let noiseless = super::reduce_noise(&src);
        if let Ok(s) = noiseless {
            assert!(s.starts_with("blob#!#$2"));
            assert!(UUID_RE.is_match(&s));
        } else { panic!("Poor blob, we knew it well. Expected blob#!#$2 but got {noiseless:?}") }
    }

    #[test]
    fn slug_and_reduce() {
        let _ = env_logger::try_init();
        let src = "blob#!!#$$2";
        let noiseless = src.slugify().reduce_noise();
        if let Ok(s) = noiseless {
            assert_eq!("blob_", s);
        } else { panic!("Oh noes… got: {noiseless:?}")};
    }

    #[test]
    fn as_id_and_reduce() {
        let _ = env_logger::try_init();
        let src = "blob#!!#$$2";
        let noiseless = src.as_id()
            .expect(&format!("Oh dear... '{src}' didn't survive as_id()?!"))
            .reduce_noise();
        if let Ok(s) = noiseless {
            assert_eq!("blob_2", s);
        } else { panic!("Oh noes… got: {noiseless:?}")};
    }
}
