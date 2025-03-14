//! The `list-trees` command.

use crate::{Context, DsError, Result};

pub fn command(ctx: &Context) -> Result {
    if ctx.config.trees.is_empty() {
        return Err(DsError::NothingToList);
    }

    let trees = ctx.config.trees.clone();
    for (name, tree) in trees {
        println!("{} Tree:", name.0);
        // TODO: Create a pretty printer for the Tree.
        // like
        //
        // TmuxVSplit:
        //   | lhs: Cmd(hx)
        //   | rhs: TmuxHSplit:
        //      |  top  : TMuxDefault
        //      | bottom: TMuxDefault
        println!("  {:?}", tree);
        println!();
    }
    Ok(())
}
