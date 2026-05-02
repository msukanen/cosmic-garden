use async_trait::async_trait;
use crate::{cmd::{Command, CommandCtx}};

pub struct DescCommand;

#[async_trait]
impl Command for DescCommand {
    async fn exec(&self, ctx: &mut CommandCtx<'_>) {
        cmd_xedit_desc!(self, ctx, medit, "MEdit");
    }
}

#[cfg(test)]
mod medit_desc_tests {
    use std::{io::Cursor, time::Duration};

    use crate::{cmd::medit::{MeditCommand, desc::DescCommand, iex::IexCommand, rename::RenameCommand}, ctx, get_operational_mock_librarian, get_operational_mock_life, stabilize_threads, thread::{SystemSignal, signal::SpawnType}, util::access::Access, world::world_tests::get_operational_mock_world};

    #[tokio::test]
    async fn medit_desc() {
        let mut b: Vec<u8> = vec![];
        let mut s = Cursor::new(&mut b);
        let (w,c,(state, p),_) = get_operational_mock_world().await;
        let _ = get_operational_mock_life!(c,w);
        let _ = get_operational_mock_librarian!(c,w);
        stabilize_threads!(150);
        let c = c.out;
        let sup = false;
        c.life.send(SystemSignal::Spawn { what: SpawnType::Mob { id: "goblin".into() }, room: "r-1".into(), reply: None }).ok();
        tokio::time::sleep(Duration::from_millis(75)).await;
        let state = ctx!(sup sup, state, MeditCommand, "goblin", s,c,w,|out:&str| out.contains("Huh?"));
        p.write().await.access = Access::Builder;
        let state = ctx!(sup sup, state, MeditCommand, "goblin", s,c,w,|out:&str| out.contains("nvoked"));
        let state = ctx!(sup sup, state, RenameCommand, "Morg-Gluglug",s,c,w,|out:&str| out.contains("renamed"));
        let state = ctx!(state, DescCommand, "v=The little googolplex goblin!",s,c,w);
        let _ = ctx!(state, IexCommand, "",s,c,w);
    }
}
