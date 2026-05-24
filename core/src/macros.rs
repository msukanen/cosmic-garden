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
        // Full output, no assert.
        ($state:ident, $cmd:ident, $args:expr, $mock_sock:ident, $sigs:expr, $world:ident) => {{
            crate::ctx!($state,$cmd,$args,$mock_sock,$sigs,$world,|_|true)
        }};

        // Full output + assert.
        ($state:ident, $cmd:ident, $args:expr, $mock_sock:ident, $sigs:expr, $world:ident, $assert:expr) => {{
            crate::ctx!(xsup false,$state,$cmd,$args,$mock_sock,$sigs,$world,$assert)
        }};

        // maybe sup spam? no assert
        (xsup $sup:expr, $state:ident, $cmd:ident, $args:expr, $mock_sock:ident, $sigs:expr, $world:ident) => {{
            crate::ctx!(abrt false sup $sup,$state,$cmd,$args,$mock_sock,$sigs,$world,|_|true)
        }};

        // suppress output, no assert
        (sup $state:ident, $cmd:ident, $args:expr, $mock_sock:ident, $sigs:expr, $world:ident) => {{
            crate::ctx!(abrt false sup true,$state,$cmd,$args,$mock_sock,$sigs,$world,|_|true)
        }};

        // maybe sup spam? + assert
        (xsup $sup:expr, $state:ident, $cmd:ident, $args:expr, $mock_sock:ident, $sigs:expr, $world:ident, $assert:expr) => {{
            crate::ctx!(abrt false sup $sup,$state,$cmd,$args,$mock_sock,$sigs,$world,$assert)
        }};

        // suppress output, do assert
        (sup $state:ident, $cmd:ident, $args:expr, $mock_sock:ident, $sigs:expr, $world:ident, $assert:expr) => {{
            crate::ctx!(abrt false sup true,$state,$cmd,$args,$mock_sock,$sigs,$world,$assert)
        }};

        // abrt instead of panic? sup "spam"?
        (abrt $abrt:literal sup $sup:expr, $state:ident, $cmd:ident, $args:expr, $mock_sock:ident, $sigs:expr, $world:ident, $assert:expr) => {{
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

/// Start a mock broadcast and such listener.
#[cfg(test)]
#[macro_export]
macro_rules! start_mock_broadcast_listener {
    ($sigs:expr) => {{
        // report RSS every 2nd sec
        let mut rss_report_interval = tokio::time::interval(std::time::Duration::from_secs(2));
        let mut sys = sysinfo::System::new_all();
        let pid = sysinfo::get_current_pid().expect("Unable to determine PID?!");
        let mut peak_mem_kb: u64 = 0;
        let mut peak_counted = 0;
        let mut rx = $sigs.out.broadcast.subscribe();
        tokio::spawn(async move {
            log::debug!("Broadcast listener starting…");
            loop {
                tokio::select! {
                    res = rx.recv() => match res {
                        Ok(b) => match b {
                            crate::io::broadcast::Broadcast::MessageInRoom2
                                { message_actor, message_other, .. } => {
                                    log::debug!("\n  → {message_actor}\n  → {message_other}");
                                },
                            crate::io::broadcast::Broadcast::BattleMessage3
                                { message_atk, message_other, message_vct, ..} => {
                                    log::debug!("  atk: \"{message_atk}\"");
                                    log::debug!("  vct: \"{message_vct}\"");
                                    log::debug!("other: \"{message_other}\"");
                                },
                            crate::io::broadcast::Broadcast::MessageSelf
                                { to, message } => {
                                    log::debug!("\nhunger: {to:?}\n{message}");
                                }
                            _ => {},
                        }
                        _ => {/* ignore errors */}
                    },

                    _ = rss_report_interval.tick() => {
                        sys.refresh_memory();
                        if let Some(process) = sys.process(pid) {
                            peak_counted += 1;
                            let curr_mem_use = process.memory();
                            let kib = peak_mem_kb as f64 / 1024.0;
                            let mib = kib / 1024.0;
                            let gib = mib / 1024.0;
                            if curr_mem_use > peak_mem_kb {
                                peak_mem_kb = curr_mem_use;
                                log::info!("[TELEMETRY] peak mem usage: {gib:.2}GB ({mib:.2}MB; {kib:.2}KB)");
                                if gib > 40.0 {
                                    log::warn!("[CRITICAL] Garden is occupying >40GB RAM.");
                                }
                            } else {
                                if peak_counted % 2 == 0 {
                                    log::trace!("[MEM] usage: {gib:.2}GB ({mib:.2}MB; {kib:.2}KB)");
                                }
                            }
                        }
                    }
                }
            }
        })
    }};
}

/// Should something pulse?
#[macro_export]
macro_rules! should_pulse {
    // true|false
    ($now:ident, $earlier:expr, $tick_id:expr, $modulo:expr) => {{
        ($tick_id.wrapping_add($now) % $modulo == 0)
            || ($now - $earlier > $modulo)
    }};

    // ()-return, record matching pulse
    (ret $now:ident, $earlier:expr, $tick_id:expr, $modulo:expr) => {{
        crate::should_pulse!(if_not (); $now, $earlier, $tick_id, $modulo)
    }};

    // $val return; record matching pulse
    (if_not $val:expr; $now:ident, $earlier:expr, $tick_id:expr, $modulo:expr) => {{
        if ($tick_id.wrapping_add($now) % $modulo == 0)
            || ($now - $earlier > $modulo) {
            $earlier = $now;
            true
        } else {
            return $val;
        }
    }};
}
