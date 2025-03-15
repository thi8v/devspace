//! The `edit` command.

use std::path::PathBuf;

use crate::{Context, DsError, Result, config::SpaceTreeId};

pub fn command(
    ctx: &mut Context,
    space_name: String,
    wdir: Option<PathBuf>,
    tree: Option<SpaceTreeId>,
) -> Result {
    let space = ctx.db.get_space_mut(&space_name)?;
    let old_space = space.clone();

    if let Some(wdir) = wdir {
        if !wdir.exists() {
            return Err(DsError::DirDoesntExists(wdir));
        }
        space.wdir = wdir;
    }

    if let Some(tree) = tree {
        if ctx.config.get_tree(&tree).is_err() {
            return Err(DsError::SpaceTreeNotFound(tree));
        }
        space.tree = tree;
    }

    println!(
        "from {:?}, {}, {}",
        space_name,
        old_space.wdir.display(),
        old_space.tree.0
    );

    println!(
        "to   {:?}, {}, {}",
        space_name,
        space.wdir.display(),
        space.tree.0
    );

    Ok(())
}
