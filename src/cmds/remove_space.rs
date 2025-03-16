//! The `remove` command.

use crate::{Context, DsError, Result};

pub fn command(ctx: &mut Context, space: String) -> Result {
    if ctx.db.get_space(&space).is_err() {
        return Err(DsError::SpaceAlreadyExists(space));
    }

    ctx.db.remove(&space);

    Ok(())
}
