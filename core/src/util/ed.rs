use std::num::{IntErrorKind, NonZeroU32, NonZeroUsize, ParseIntError};

use crate::{string::{newline::LineEndingExt, styling::{MAX_DESCRIPTION_LINES, RULER_LINE}}, tell_user};

#[derive(Debug)]
pub enum EditorError {
    MaxLineCount,
    TooLong,
    ParseIntError(ParseIntError),
}

impl std::error::Error for EditorError {}
impl std::fmt::Display for EditorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MaxLineCount => write!(f, "Max line count of {MAX_DESCRIPTION_LINES} exceeded."),
            Self::TooLong => write!(f, "Max text length of {} characters would have been exceeded.", 79 * MAX_DESCRIPTION_LINES),
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

/// A versatile text editing function.
/// 
/// Modus operandi is determined by the first character in `args`.
/// 
/// - `+` — insert line.
/// - `-` — remove line.
/// - `=` — ignore `source`, use `args` as full replacement.
pub async fn edit_text(writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send), args: &str, source: &str) -> Result<EdResult, EditorError> {
    let source = source.trim_end();
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
    
    let op = args.chars().next();
    if let Some(c @ ('r'|'+')) = op {
        //
        // '+' -- insert as specified line…
        // 'r' -- replace the specified line…
        //
        let args = args[1..].trim_start().splitn(2, ' ').collect::<Vec<&str>>();
        let lno = match args[0].parse::<usize>() {
            Ok(lno) if lno > MAX_DESCRIPTION_LINES => {
                tell_user!(writer,
                            "<c red>Warning!</c> Maximum help entry description length is limited to {} lines.\n\
                            Command cancelled — no changes made.\n",
                            MAX_DESCRIPTION_LINES);
                return Err(EditorError::MaxLineCount)
            }
            Ok(lno) => lno,
            Err(e) => {
                return {
                    tell_user!(writer, "<c red>Error! </c>{:?}\n", e);
                    Err(EditorError::ParseIntError(e))
                };
            }
        };

        let new_line = args.get(1).unwrap_or(&"");
        let text = match c {
            'r' => replace_nth_line(&source, lno, new_line),
            '+' => insert_nth_line(&source, lno, new_line),
            _ => unreachable!()
        };
        let lno = text.lines().count();
        if lno > MAX_DESCRIPTION_LINES {
            tell_user!(writer, "Nope, not doing — too many lines {lno}. TL;DR… already at {}!\n", MAX_DESCRIPTION_LINES);
            return Err(EditorError::MaxLineCount);
        }
        if text.len() > 79*MAX_DESCRIPTION_LINES && lno < MAX_DESCRIPTION_LINES {
            tell_user!(writer, "Nope, not doing — text too cramped. Rewrite better…\n");
            return Err(EditorError::TooLong);
        }
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
    let mut lf_ensured = source.ensure_lf();
    if lf_ensured.lines().count() >= MAX_DESCRIPTION_LINES {
        tell_user!(writer, "ERR - there's already {}+ lines of text… Can't add more.\n", MAX_DESCRIPTION_LINES);
        return Err(EditorError::MaxLineCount);
    }
    if lf_ensured.len() + args.len() >= 79 * MAX_DESCRIPTION_LINES {
        tell_user!(writer, "ERR - try to be more conscise. Description would exceed {} characters in length…\n", 79*MAX_DESCRIPTION_LINES);
        return Err(EditorError::TooLong);
    }
    lf_ensured.push_str(&format!("{}\n", args));
    if !verbose {
        tell_user!(writer, "OK - text appended.\n");
    }
    Ok(EdResult::ContentReady { text: lf_ensured, dirty: true, verbose })
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
        lines.push(text_to_insert.trim_end());
    } else {
        lines.insert(index, text_to_insert.trim_end());
    }

    format!("{}\n", lines.join("\n"))
}

/// Replaces a line in a string string at the nth position (1-based index).
fn replace_nth_line(text: &str, line_num: usize, text_to_insert: &str) -> String {
    if line_num == 0 {
        return text.to_string(); // Or handle as an error
    }

    let mut lines: Vec<&str> = text.lines().collect();
    let index = (line_num - 1).min(lines.len() + 10);

    if index >= lines.len() {
        while lines.len() < index {
            lines.push("");
        }
        lines.push(text_to_insert.trim_end());
    } else {
        lines[index] = text_to_insert.trim_end();
    }

    format!("{}\n", lines.join("\n"))
}

/// Access `$ed` (hedit, redit, etc.) of the given `$ctx`.
/// 
/// # Args
/// - `$plr` Arc<RwLock>>
/// - `$ed` e.g. hedit, iedit, redit …
// NOTE: may sputter with Enum based stuff, like Iedit.
#[macro_export]
macro_rules! access_ed_entry {
    ($plr:ident, $ed:ident) => {
        paste::paste! {
            $plr.read().await.[<$ed _buffer>].as_ref().unwrap()
        }
    };
}

#[cfg(test)]
mod ed_tests {
    use std::io::Cursor;

    use crate::{cmd::{look::LookCommand, redit::{ReditCommand, desc::DescCommand}}, io::ClientState, string::{Describable, DescribableMut, newline::LineEndingExt, styling::MAX_DESCRIPTION_LINES}, util::access::Access, world::world_tests::get_operational_mock_world};

    #[test]
    fn remove_nth_line() {
        use super::*;
        let text = "This text has\n3 lines.\nAt least before removal of line #2.";
        let r = remove_nth_line(text, "2");
        if let Ok((true, res)) = r {
            assert_eq!("This text has\nAt least before removal of line #2.", res.as_str());
        } else {
            panic!("No go!");
        }
    }

    #[tokio::test]
    async fn ed_79_21_plus() {
        let mut b: Vec<u8> = vec![];
        let mut s = Cursor::new(&mut b);
        let (w,c,p,d) = get_operational_mock_world().await;
        let c = c.out;
        let l79: &'static str = "0123456789012345678901234567890123456789012345678901234567890123456789012345678\n";
        let mut l21 = String::new();
        for _ in 0..21 {
            l21.push_str(l79);
        }
        assert_eq!(80*MAX_DESCRIPTION_LINES, l21.len());
        let lined = (&*l21).ensure_lf();
        assert_eq!(MAX_DESCRIPTION_LINES, lined.lines().count());
        let l80: &'static str = "01234567890123456789012345678901234567890123456789012345678901234567890123456789";
        let mut l80_21 = String::new();
        for _ in 0..21 {
            l80_21.push_str(l80);
        }
        let lined = (&*l80_21).ensure_lf();
        let len = l80_21.len();
        assert_eq!(80*MAX_DESCRIPTION_LINES, len);
        let lno = lined.lines().count();
        // within limits even if cramped?
        if lno < MAX_DESCRIPTION_LINES && len > 79*MAX_DESCRIPTION_LINES {
            // this should log an error
            log::error!("Text cramped: {lno}L >= {MAX_DESCRIPTION_LINES}max or {len}C > {}C", 79*MAX_DESCRIPTION_LINES);
        } else {
            panic!("Hey!? Where's my error log for lno={lno} len={len}?");
        }
        // lets fool around with r-1's description…
        let r1 = if let Some(r1) = w.read().await.rooms.get("r-1") {
            r1.clone()
        } else { panic!("r-1 poofed?") };
        r1.write().await.set_desc(&l21);
        let state = ClientState::Playing { player: p.clone() };
        p.write().await.access = Access::Builder;
        let state = ctx!(state, LookCommand, "", s,c,w,p,|out:&str| out.contains("01234"));
        let state = ctx!(state, ReditCommand, "here", s,c,w,p);
        let state = ctx!(state, DescCommand, "Maybe...",s,c,w,p,|out:&str| out.contains("ERR"));
        let state = ctx!(state, DescCommand, "v+21 Maybe...",s,c,w,p,|out:&str| out.contains("too many lines"));
        let state = ctx!(state, DescCommand, "vr21 Maybe...",s,c,w,p,|out:&str| out.contains("Maybe..."));
        let state = ctx!(state, DescCommand, &format!("vr22 {}", l79),s,c,w,p,|out:&str| out.contains("Maximum help"));
        let _ = ctx!(state, DescCommand, &format!("vr21 {}", l79),s,c,w,p,|out:&str| !out.contains("Maybe"));
    }
}
