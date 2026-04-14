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
    macro_rules! ctx {
        ($cmd:ident, $args:literal, $mock_sock:ident, $tx:ident, $world:ident, $plr:ident) => {
            {
            let mut ctx = CommandCtx {
                writer: &mut $mock_sock,
                args: $args,
                pre_pad_n: false,
                state: ClientState::Playing { player: $plr.clone() },
                world: $world.clone(),
                tx: &$tx
            };
            $cmd.exec(&mut ctx).await;
            }
            log::debug!("{}", String::from_utf8_lossy($mock_sock.get_ref()));
        };
    }
