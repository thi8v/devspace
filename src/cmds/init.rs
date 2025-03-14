//! The `init` command

use std::{fs::canonicalize, path::PathBuf};

use crate::{Context, DsError, Result, config::SpaceTreeId, database::Space};

pub fn command(ctx: &mut Context, path: PathBuf, tree: Option<SpaceTreeId>) -> Result {
    let abs = canonicalize(path)?;

    let dir_name = abs
        .file_name()
        .expect("the path of the directory can't finish with `..`")
        .to_string_lossy();

    if ctx.db.get_space(&dir_name).is_ok() {
        return Err(DsError::SpaceAlreadyExists(dir_name.into_owned()));
    }

    ctx.db.insert(
        dir_name.into_owned(),
        Space::new(abs, tree.unwrap_or(ctx.config.default_tree.clone())),
    );

    Ok(())
}
