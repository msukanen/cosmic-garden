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
    /// CommandCtx utilizing macro.
    /// 
    /// Requires /src/test/world_test_harness.inc contents in the test fn.
    /// 
    /// # Args
    /// - `cmd`
    /// - `args`
    /// - `mock_sock`
    /// - `tx`
    /// - `sigs`
    /// - `world`
    /// - `plr`
    /// - `assert`ion expr
    /// 
    /// # Examples
    /// - `ctx!(IeditCommand, "apple", mock_sock, tx, world, plr);`
    /// - `ctx!(IeditCommand, "apple", mock_sock, tx, world, plr, |out:&str| out.contains("apple"));`
    macro_rules! ctx {
        ($state:ident, $cmd:ident, $args:literal, $mock_sock:ident, $tx:ident, $sigs:ident, $world:ident, $plr:ident) => {{
            crate::ctx!($state,$cmd,$args,$mock_sock,$tx,$sigs,$world,$plr,|_|true)
        }};

        ($state:ident, $cmd:ident, $args:literal, $mock_sock:ident, $tx:ident, $sigs:ident, $world:ident, $plr:ident, $assert:expr) => {{
            let state = {
                use crate::cmd::{Command,CommandCtx};
                $mock_sock.get_mut().clear();
                let mut ctx = CommandCtx {
                    writer: &mut $mock_sock,
                    args: $args,
                    pre_pad_n: false,
                    system: &$sigs.0,
                    state: $state,
                    world: $world.clone(),
                    tx: &$tx
                };
                $cmd.exec(&mut ctx).await;
                ctx.state.clone()
            };
            // some assertions to do… maybe.
            {
                let out = String::from_utf8_lossy($mock_sock.get_ref());
                assert!($assert(&out), "Ass fail! Out was '{}'", out.trim_end());
            }
            log::debug!("\n{}", String::from_utf8_lossy($mock_sock.get_ref()));
            state
        }};
    }
