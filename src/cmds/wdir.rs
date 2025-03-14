//! The `wdir` command.

use crate::{Context, Result};

pub fn command(ctx: &Context, space_name: String) -> Result {
    let space = ctx.db.get_space(&space_name)?;

    println!("{}", space.wdir.to_string_lossy());
    Ok(())
}
