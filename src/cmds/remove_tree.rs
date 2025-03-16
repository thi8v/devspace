//! The `remove-tree` command.

use crate::{Context, Result, config::SpaceTreeId};

pub fn command(ctx: &mut Context, tree_name: String) -> Result {
    // because we propagate the error the check is still performed.
    ctx.config.get_tree(&SpaceTreeId(tree_name.clone()))?;

    ctx.config.remove_tree(tree_name);
    Ok(())
}
