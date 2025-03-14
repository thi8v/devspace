//! The `go` command.

use tmux_interface::{AttachSession, HasSession, NewSession, StdIO, Tmux, TmuxCommands};

use crate::{Context, Result};

pub fn command(ctx: &mut Context, space_name: String) -> Result {
    let session_name = ctx.session_name(&space_name);
    let space = ctx.db.get_space(&space_name)?;

    let session_exists = Tmux::with_command(HasSession::new().target_session(&session_name))
        .output()?
        .success();

    // the session already exists, don't create another one just attach to it.
    if session_exists {
        let _ = Tmux::with_command(AttachSession::new().target_session(&session_name))
            .stdin(Some(StdIO::Inherit))
            .stdout(Some(StdIO::Inherit))
            .stderr(Some(StdIO::Inherit))
            .output()?;

        return Ok(());
    }

    let mut cmds = TmuxCommands::new().add_command(
        NewSession::new()
            .attach()
            .session_name(&session_name)
            .start_directory(space.wdir.to_string_lossy())
            .into(),
    );

    let tree = ctx.config.get_tree(&space.tree)?;
    let session_name = &ctx.session_name(&space_name);
    let built_treee = tree.build(space, session_name)?;

    cmds.push_cmds(built_treee);

    let _ = Tmux::with_commands(cmds)
        .stdin(Some(StdIO::Inherit))
        .stdout(Some(StdIO::Inherit))
        .stderr(Some(StdIO::Inherit))
        .output()?;

    Ok(())
}
