//! The `list-trees` command.

use std::io::Write;

use crate::{Context, DsError, Result, config::SpaceTree};

pub fn command(ctx: &Context) -> Result {
    if ctx.config.trees.is_empty() {
        return Err(DsError::NothingToList);
    }
    let mut stdout = std::io::stdout();

    let mut trees = ctx.config.trees.iter().collect::<Vec<_>>();
    trees.sort_by(|a, b| a.0.0.cmp(&b.0.0));

    writeln!(stdout, "List of trees:")?;
    for (name, tree) in trees {
        writeln!(stdout)?;
        write!(stdout, "{:?}:\n  ", name.0)?;
        tree.pretty_print(&mut stdout, SpaceTree::PRINT_INDENT)?;
    }

    stdout.flush()?;

    Ok(())
}
