//! Editor modes for those who need them.

pub mod mode; pub use mode::*;

#[macro_export]
macro_rules! validate_editor_mode {
    ($ctx:ident, $mode:literal) => {{
        if !$ctx.state.is_editing() {
            crate::err_tell_user!($ctx.writer, "Invoke {} first…\n", $mode);
        }
    }};
}
