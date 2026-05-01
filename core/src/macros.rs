//! Macross (yes, not a typo) file to appease the braindead #[macro_export] rulings...

/// Show a help topic and bail out.
#[macro_export]
macro_rules! show_help {
    ($ctx:ident, $topic:expr) => {{
        crate::cmd::help::HelpCommand.exec({$ctx.args = $topic; $ctx}).await;
        return;
    }};
}

/// Show a help topic if needed and bail out if afterwards.
#[macro_export]
macro_rules! show_help_if_needed {
    ($ctx:ident, $topic:expr) => {
        if $ctx.args.is_empty() || $ctx.args.starts_with('?') {
            $crate::show_help!($ctx, $topic);
        }
    };
}

#[macro_export]
macro_rules! err_iedit_buffer_inaccessible {
    ($ctx:ident, $p:ident, $p_id:ident) => {
        drop($p);
        log::error!("Builder '{}'.iedit_buffer evaporated mid-edit?!", $p_id);
        crate::tell_user!($ctx.writer, "Uh-oh, editor buffer evaporated?!\n");
        return;
    };
}

#[cfg(test)]
#[macro_export]
    /// [CommandCtx][crate::cmd::CommandCtx]<'_> utilizing macro.
    // 
    // Requires /src/test/world_test_harness.inc contents in the test fn.
    // 
    /// # Args
    /// - [abrt true|false] - ::abort() instead of panic!()
    /// - [sup true|false] - suppress "client" spam
    /// - `state` - ClientState
    /// - `cmd`
    /// - `args`
    /// - `mock_sock`
    /// - `sigs`
    /// - `world`
    /// - `plr`
    /// - `assert`ion expr
    /// 
    /// # Examples
    /// - `ctx!(state, IeditCommand, "apple", mock_sock, sigs, world, plr);`
    /// - `ctx!(state, IeditCommand, "apple", mock_sock, sigs, world, plr, |out:&str| out.contains("apple"));`
    /// - `ctx!(sup true, state, IeditCommand, "apple", mock_sock, sigs, world, plr, |out:&str| out.contains("apple"));`
    /// - `ctx!(abrt true sup true, state, IeditCommand, "apple", mock_sock, sigs, world, plr, |out:&str| out.contains("apple"));`
    macro_rules! ctx {
        ($state:ident, $cmd:ident, $args:expr, $mock_sock:ident, $sigs:expr, $world:ident, $plr:ident) => {{
            crate::ctx!($state,$cmd,$args,$mock_sock,$sigs,$world,$plr,|_|true)
        }};

        ($state:ident, $cmd:ident, $args:expr, $mock_sock:ident, $sigs:expr, $world:ident, $plr:ident, $assert:expr) => {{
            crate::ctx!(sup false,$state,$cmd,$args,$mock_sock,$sigs,$world,$plr,$assert)
        }};

        // sup "spam"?
        (sup $sup:expr, $state:ident, $cmd:ident, $args:expr, $mock_sock:ident, $sigs:expr, $world:ident, $plr:ident) => {{
            crate::ctx!(abrt false sup $sup,$state,$cmd,$args,$mock_sock,$sigs,$world,$plr,|_|true)
        }};

        // sup "spam"?
        (sup $sup:expr, $state:ident, $cmd:ident, $args:expr, $mock_sock:ident, $sigs:expr, $world:ident, $plr:ident, $assert:expr) => {{
            crate::ctx!(abrt false sup $sup,$state,$cmd,$args,$mock_sock,$sigs,$world,$plr,$assert)
        }};

        // abrt instead of panic? sup "spam"?
        (abrt $abrt:literal sup $sup:expr, $state:ident, $cmd:ident, $args:expr, $mock_sock:ident, $sigs:expr, $world:ident, $plr:ident, $assert:expr) => {{
            let state = {
                use crate::cmd::{Command,CommandCtx};
                $mock_sock.get_mut().clear();
                let mut ctx = CommandCtx {
                    writer: &mut $mock_sock,
                    args: $args,
                    pre_pad_n: false,
                    out: &$sigs,
                    state: $state,
                    world: $world.clone(),
                };
                $cmd.exec(&mut ctx).await;
                ctx.state.clone()
            };
            let out_raw = $mock_sock.get_ref();
            let out = String::from_utf8_lossy(out_raw).to_string().trim_end().to_string();
            let assert_result = $assert(&out);
            if !out.is_empty() && !$sup {
                log::debug!("\n{}", out.to_string());
            }
                if !assert_result {
                    log::error!("{}", stringify!($assert));
                    log::error!("Ridonkylous! Read above...");
                    
                    if $abrt {
                        std::process::abort();
                    }
                    panic!("Oops");
                }
            state
        }};
    }

/// Convert given [std::sync::Arc][Arc]|[std::sync::Weak][Weak] into `usize`.
#[macro_export]
macro_rules! lock2key {
    (arc $arc:expr) => {
        crate::lock2key!(weak &std::sync::Arc::downgrade(&$arc))
    };

    (weak $weak:expr) => {
        std::sync::Weak::as_ptr($weak) as *const() as usize
    }
}
