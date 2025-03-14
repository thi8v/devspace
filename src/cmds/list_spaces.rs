//! The `list-spaces` command.

use crate::{Context, DsError, Result};

pub fn command(ctx: &Context) -> Result {
    if ctx.db.is_empty() {
        return Err(DsError::NothingToList);
    }

    let mut spaces = ctx.db.spaces_iter().collect::<Vec<_>>();
    spaces.sort_by(|a, b| a.0.cmp(b.0));

    // can safely unwrap because we know there is at least one value.
    let name_width = spaces.iter().map(|(s, _)| s.len()).max().unwrap().max(8);
    let path_width = spaces
        .iter()
        .map(|(_, s)| s.wdir.to_str().unwrap().len())
        .max()
        .unwrap();
    let tree_width = spaces.iter().map(|(_, s)| s.tree.0.len()).max().unwrap();

    println!(
        "{:^name_width$}| {:^path_width$} | {:^tree_width$}",
        "NAME", "PATH", "TREE"
    );
    for (name, space) in spaces {
        println!(
            "{:name_width$}| {:path_width$} | {:tree_width$}",
            name,
            space.wdir.to_string_lossy(),
            space.tree.0
        );
    }
    Ok(())
}
