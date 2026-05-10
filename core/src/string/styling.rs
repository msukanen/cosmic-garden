use std::fmt::Display;

use ansi_term::{Colour, Style};

pub const EDITOR_DIRTY: &str = "<c red>^*</c>";
pub const MAX_DESCRIPTION_LINES: usize = 21; // a modest number, sort of fits on a tiny 80x24 terminal thingydoodah. Takes header, title, etc. into account.

/// Apply semantic theme
fn apply_semantic_theme(style: Style, payload: Option<&&str>) -> Style {
    let Some(payload) = payload else { return style };
    match *payload {
        "err" => style.fg(Colour::Red).bold(),
        "warn" => style.fg(Colour::Yellow).bold(),
        "info" => style.fg(Colour::Cyan),
        "usage" => style.fg(Colour::Green),
        "ex"|"example" => style.fg(Colour::Yellow),
        "title" => style.fg(Colour::Green).bold(),
        // unknown?
        _ => style
    }
}

/// Parses a color name string into an ansi_term::Colour.
fn parse_color(name: &str) -> Option<Colour> {
    match name.to_lowercase().as_str()
    {
        "black" => Some(Colour::Black),
        "red" => Some(Colour::Red),
        "green" => Some(Colour::Green),
        "yellow" => Some(Colour::Yellow),
        "blue" => Some(Colour::Blue),
        "purple" => Some(Colour::Purple),
        "cyan" => Some(Colour::Cyan),
        "white" => Some(Colour::White),
        // RGB stuff:
        "gray"|"grey" => Some(Colour::Fixed(8)),
        _ => None,
    }
}

/// Return dirty-marker str based on `dirty` flag.
pub const fn dirty_mark(dirty: bool) -> &'static str {if dirty {EDITOR_DIRTY} else {""}}

/// A colored ruler line for e.g. edit_text().
pub const RULER_LINE: &str = "|__<c green>.</c>\
<c cyan>5</c>___<c green>T</c>\
<c cyan>10</c>_<c green>.</c>_\
<c cyan>15</c><c green>.</c>__\
<c cyan>20</c>___\
<c cyan><c green>2</c>5</c>___\
<c cyan>30</c>___\
<c cyan>35</c>___\
<c cyan>40</c>___\
<c cyan>45</c>___\
<c cyan>50</c>___\
<c cyan>55</c>___\
<c cyan>60</c>___\
<c cyan>65</c>___\
<c cyan>70</c>___\
<c cyan>75</c>___|";
/// A boring ruler line for various uses…
pub const RULER_LINE_PLAIN: &str = "-------------------------------------------------------------------------------";

#[macro_export]
macro_rules! cformat {
    ($($arg:tt)*) => {{
        use crate::string::styling::ColorExt;
        format!($($arg)*).colored()
    }};
}
#[allow(dead_code)]// just to appease VS Code + rust analyzer for visuals
pub trait ColorExt {
    fn colored(&self) -> String;
}
impl<T: Display> ColorExt for T {fn colored(&self) -> String { format_color(&self) }}

/// Formats a string with custom color tags into an ANSI-colored string.
pub fn format_color<S: Display>(input: S) -> String {
    let input = input.to_string();
    let mut output = String::new();
    let mut style_stack = vec![Style::new()];
    
    let mut text_buffer = String::new();
    let mut tag_buffer = String::new();
    let mut in_tag = false;

    for c in input.chars() {
        if c == '<' {
            if in_tag {
                // We found a '<' while already inside a tag. This means the previous
                // '<' and the buffered tag content were literal text.
                if let Some(current_style) = style_stack.last() {
                    output.push_str(&current_style.paint(format!("<{}", tag_buffer)).to_string());
                }
                tag_buffer.clear();
            } else {
                // This is the start of a new tag. Paint any buffered text first.
                if !text_buffer.is_empty() {
                    if let Some(current_style) = style_stack.last() {
                        output.push_str(&current_style.paint(&text_buffer).to_string());
                    }
                    text_buffer.clear();
                }
            }
            in_tag = true;
        } else if c == '>' && in_tag {
            // We're ending a tag. Process it.
            in_tag = false;
            
            let tag_parts: Vec<&str> = tag_buffer.split_whitespace().collect();
            let mut tag_processed = false;

            if let Some(tag_name) = tag_parts.first() {
                let is_closing = tag_name.starts_with('/');
                let actual_tag = if is_closing { &tag_name[1..] } else { *tag_name };

                match actual_tag {
                    "c"|"bg"|"x" => {
                        tag_processed = true;
                        if is_closing {
                            if style_stack.len() > 1 {
                                style_stack.pop();
                            }
                        } else {
                            let mut new_style = style_stack.last().cloned().unwrap_or_default();
                            match actual_tag {
                                "c"|"bg" => {
                                    if let Some(color_name) = tag_parts.get(1) {
                                        if let Some(color) = parse_color(color_name) {
                                            match *tag_name {
                                                "c" => new_style = new_style.fg(color),
                                                "bg" => new_style = new_style.on(color),
                                                _ => {}
                                            }
                                        }
                                    }
                                }

                                "x" => {
                                    new_style = apply_semantic_theme(new_style, tag_parts.get(1));
                                    // TODO: fetch preconfigured const
                                }

                                _ => unreachable!("Handled in outer match.")
                            }
                            style_stack.push(new_style);
                        }
                    }
                    ,_=> ()//log::warn!("Unknown tag <{}{actual_tag}>", if is_closing {"/"} else {""})
                }
            }

            if !tag_processed {
                // Not a valid tag, treat it as literal text.
                if let Some(current_style) = style_stack.last() {
                    output.push_str(&current_style.paint(format!("<{}>", tag_buffer)).to_string());
                }
            }
            tag_buffer.clear();

        } else if in_tag {
            tag_buffer.push(c);
        } else {
            text_buffer.push(c);
        }
    }

    // Append any remaining text
    if !text_buffer.is_empty() {
        if let Some(current_style) = style_stack.last() {
            output.push_str(&current_style.paint(&text_buffer).to_string());
        }
    }
    // Handle unterminated tag
    if !tag_buffer.is_empty() {
        if let Some(current_style) = style_stack.last() {
            output.push_str(&current_style.paint(format!("<{}", tag_buffer)).to_string());
        }
    }

    output
}

///
/// Maybe 's', based on `num`.
/// 
pub const fn maybe_plural(num: i32) -> &'static str { if num == 1 {""} else {"s"}}

pub trait Truthy {
    fn true_false(&self) -> bool;
}

impl Truthy for str {
    fn true_false(&self) -> bool {
        if self.is_empty() { false }
        else { match self.chars().nth(0).unwrap() {
            'f'|'F'|'n'|'N'|'0' => false,
            _ => true
        }}
    }
}

#[cfg(test)]
mod ansi_tests {
    #[test]
    fn format_color() {
        let _ = env_logger::try_init();
        let now = std::time::Instant::now();
        let input_string = "This is <c yellow>Yellow text <bg cyan>on cyan bg</bg> which continues as yellow</c>, until it doesn't.";
        
        // log::debug!("--- Input String ---");
        // log::debug!("{}", input_string);
        
        // log::debug!("\n--- Formatted Output ---");
        let fmt = super::format_color(input_string);
        // log::debug!("{fmt}");

        let tricky_strings = [
            "<c green>Usage:</c> force <c blue>[-]</c> <c cyan><TARGET> <COMMAND <c blue>[ARGS]</c>></c>",
            "<c red>Err:<c white> force <c blue>[-]</c> <c cyan><TARGET> <COMMAND <c blue>[ARGS]</c>></c></c></c>",
            "<c blue>***:<c gray> force <c green>[-]</c> <c cyan><bg blue><TARGET> <COMMAND</bg> <c blue>[ARGS]</c>></c></c></c>",
        ];
        // log::debug!("\n--- Tricky String ---");
        tricky_strings.iter().for_each(|s| {
            let fmt = super::format_color(s);
            // log::debug!("{fmt}");
        });
        let elapsed = now.elapsed();
        log::debug!("Elapsed {elapsed:?}");
    }
}
