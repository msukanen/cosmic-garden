use std::num::{IntErrorKind, NonZeroU32, NonZeroUsize, ParseIntError};

use tokio::net::tcp::OwnedWriteHalf;

use crate::{string::{styling::{MAX_DESCRIPTION_LINES, RULER_LINE}, LineEndingExt}, tell_user};

#[derive(Debug)]
pub enum EditorError {
    MaxLineCount,
    ParseIntError(ParseIntError),
}

impl std::error::Error for EditorError {}
impl std::fmt::Display for EditorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MaxLineCount => write!(f, "Max line count of {MAX_DESCRIPTION_LINES} exceeded."),
            Self::ParseIntError(e) => write!(f, "Numeric error {}", e),
        }
    }
}

pub enum EdResult {
    HelpRequested,
    NoChanges(bool),
    ContentReady { text: String, dirty: bool, verbose: bool },
}

impl From<ParseIntError> for EditorError { fn from(value: ParseIntError) -> Self { Self::ParseIntError(value)}}

pub trait Editor {
    fn set_description(&mut self, desc: &str);
}

/// A versatile text editing function.
/// 
/// Modus operandi is determined by the first character in `args`.
/// 
/// - `+` — insert line.
/// - `-` — remove line.
/// - `=` — ignore `source`, use `args` as full replacement.
pub async fn edit_text(writer: &mut OwnedWriteHalf, args: &str, source: &str) -> Result<EdResult, EditorError> {
    if args.is_empty() {
        return {
            let mut display = String::new();
            display.push_str(&format!("    <c gray>_</c>{RULER_LINE}\n"));
            for (idx, line) in source.ensure_lf().lines().enumerate() {
                display.push_str(&format!("<c gray>{:02} | </c>{line}\n", idx + 1));
            }
            tell_user!(writer, "{}<c red>// END</c>\n", display);
            Ok(EdResult::NoChanges(false))
        };
    }

    if args.starts_with('?') {
        return Ok(EdResult::HelpRequested);
    }

    let mut args = args;
    let mut verbose = false;
    if args.starts_with('v') {
        verbose = true;
        args = &args[1..];
    }
    
    //
    // '+' -- insert as specified line…
    //
    if args.starts_with('+') {
        let args = args[1..].trim_start().splitn(2, ' ').collect::<Vec<&str>>();
        let lno = args[0].parse::<usize>();
        let lno = match lno {
            Ok(lno) => {
                if lno > MAX_DESCRIPTION_LINES {
                    return {
                        tell_user!(writer,
                            "<c red>Warning!</c> Maximum help entry description length is limited to {} lines.\n\
                            Command cancelled — no changes made.\n",
                            MAX_DESCRIPTION_LINES);
                        Err(EditorError::MaxLineCount)
                    };
                }
                lno
            },
            Err(e) => {
                return {
                    tell_user!(writer, "<c red>Error! </c>{:?}\n", e);
                    Err(EditorError::ParseIntError(e))
                };
            }
        };

        let text = insert_nth_line(&source, lno, if args.len() < 2 {""} else {args[1]});
        return Ok(EdResult::ContentReady { text, dirty: true, verbose });
    }

    //
    // '-' -- remove a line …
    //
    if args.starts_with('-') {
        let (text, dirty) = {
            let res = remove_nth_line(&source, &args[1..]);
            let ed_dirty;
            let mut text: String;
            match res {
                Ok((dirty, desc)) => {
                    ed_dirty = dirty;
                    if dirty {
                        text = desc;
                        text.push_str("\n");
                    } else {
                        return {
                            tell_user!(writer, "Nothing to change — not that many lines to begin with.\n");
                            Ok(EdResult::NoChanges(verbose))
                        };
                    }
                },
                Err(e) => return {
                    match e.kind() {
                        IntErrorKind::PosOverflow => {tell_user!(writer, "Well, there's not quite that many lines to begin with …\n");},
                        IntErrorKind::Zero => {tell_user!(writer, "Err, line numbers generally are counted from 1 (one) and up …\n");},
                        _ => {tell_user!(writer, "That's not a valid line number, Dave.\n");}
                    }
                    Err(EditorError::ParseIntError(e))
                }
            }
            (text, ed_dirty)
        };
        
        return if !dirty {
            Ok(EdResult::NoChanges(verbose))
        } else {
            Ok(EdResult::ContentReady { text, dirty, verbose })
        }
    }
    
    //
    // '=' -- full replace
    //
    if args.starts_with('=') {
        if !verbose {
            tell_user!(writer, "OK - description replaced.\n");
        }
        return Ok(EdResult::ContentReady { text: format!("{}\n", &args[1..]), dirty: true, verbose });
    }
      
    //
    // Append at end if no sub-command specified.
    //
    let mut text = source.to_string();
    text.push_str(&format!("{}\n", args));
    if !verbose {
        tell_user!(writer, "OK - text appended.\n");
    }
    Ok(EdResult::ContentReady { text, dirty: true, verbose })
}

/// Removes nth line from given `text`.
/// 
/// # Arguments
/// - `text`— text to work with.
/// - `lno_str`— line number, as a string representation.
/// 
/// # Returns
/// 1. `(true, `modified-text`)` — if changes were made.
/// 2. `(false, `original-text`)` — if no changes done.
fn remove_nth_line(text: &str, lno_str: &str) -> Result<(bool, String), ParseIntError> {
    let lno: usize = lno_str.parse()?;
    let lno = NonZeroUsize::new(lno).ok_or_else(||"0".parse::<NonZeroU32>().unwrap_err())?;
    let lno: usize = lno.into();

    if text.lines().count() < lno {
        return Ok((false, text.into()));
    }

    Ok((true, text.lines()
        .enumerate()
        // Keep all lines where the index (0-based) is NOT the one we want to remove (1-based)
        .filter(|(i, _)| *i != lno - 1)
        // Discard the index and just keep the line's text
        .map(|(_, line)| line)
        // Collect the remaining lines into a Vec<&str> and join them with newlines
        .collect::<Vec<&str>>()
        .join("\n")))
}

/// Inserts a new line into a string at the nth position (1-based index).
fn insert_nth_line(text: &str, line_num: usize, text_to_insert: &str) -> String {
    if line_num == 0 {
        return text.to_string(); // Or handle as an error
    }

    let mut lines: Vec<&str> = text.lines().collect();
    let index = (line_num - 1).min(lines.len() + 10);

    if index >= lines.len() {
        while lines.len() < index {
            lines.push("");
        }
        lines.push(text_to_insert);
    } else {
        lines.insert(index, text_to_insert);
    }

    format!("{}\n", lines.join("\n"))
}

/// Access `$ed` (hedit, redit, etc.) of the given `$ctx`.
#[macro_export]
macro_rules! access_ed_entry {
    ($plr:ident, $ed:ident) => {
        $plr.read().await.$ed.as_ref().unwrap()
    };
}

#[cfg(test)]
mod desc_tests {
    #[test]
    fn test_remove_nth_line() {
        use super::*;
        let text = "This text has\n3 lines.\nAt least before removal of line #2.";
        let r = remove_nth_line(text, "2");
        if let Ok((true, res)) = r {
            assert_eq!("This text has\nAt least before removal of line #2.", res.as_str());
        } else {
            panic!("No go!");
        }
    }
}
