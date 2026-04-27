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

    use crate::{cmd::medit::{MeditCommand, desc::DescCommand, iex::IexCommand, rename::RenameCommand}, ctx, get_operational_mock_librarian, get_operational_mock_life, io::ClientState, thread::{SystemSignal, signal::SpawnType}, util::access::Access, world::world_tests::get_operational_mock_world};

    #[tokio::test]
    async fn medit_desc() {
        let mut b: Vec<u8> = vec![];
        let mut s = Cursor::new(&mut b);
        let (w,c,p,_) = get_operational_mock_world().await;
        let _ = get_operational_mock_life!(c,w);
        let _ = get_operational_mock_librarian!(c,w);
        tokio::time::sleep(Duration::from_secs(1)).await;
        let c = c.out;
        c.life.send(SystemSignal::Spawn { what: SpawnType::Mob { id: "goblin".into() }, room_id: "r-1".into() }).ok();
        tokio::time::sleep(Duration::from_millis(75)).await;
        let state = ClientState::Playing { player: p.clone() };
        let state = ctx!(sup true, state, MeditCommand, "goblin", s,c,w,p,|out:&str| out.contains("Huh?"));
        p.write().await.access = Access::Builder;
        let state = ctx!(sup true, state, MeditCommand, "goblin", s,c,w,p,|out:&str| out.contains("nvoked"));
        let state = ctx!(sup true, state, RenameCommand, "Morg-Gluglug",s,c,w,p,|out:&str| out.contains("renamed"));
        let state = ctx!(state, DescCommand, "v=The little googolplex goblin!",s,c,w,p);
        let _ = ctx!(state, IexCommand, "",s,c,w,p);
    }
}
