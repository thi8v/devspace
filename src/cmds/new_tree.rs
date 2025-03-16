//! The `new-tree` command.

use std::{
    io::{Stdin, Write, stdin, stdout},
    num::ParseIntError,
};
use thiserror::Error;

use crate::{Context, DsError, Result, config::SpaceTree};

/// An error when running the `new-tree` command.
#[derive(Error, Debug)]
pub enum InteractiveError {
    #[error("failed to parse your integer: {0}")]
    InvalidInt(#[from] ParseIntError),
    #[error("you provided {0} but the number should've been between 1 and 3.")]
    UnknownTreeNumber(usize),
}

pub fn command(ctx: &mut Context, tree_name: String) -> Result {
    // TODO: maybe validate if the tree name is Rust identifier like for ease
    // of use.
    // TODO: share the buffer and clear it at the start of the `new_*_tree`
    // functions.
    // TODO: if there is an error in the `new-tree` command, don't propagate it
    // with ? instead re-run what has failed.
    let mut stdout = stdout();
    let stdin = stdin();

    writeln!(stdout, "Interractive tree creation tool.\n")?;

    let tree = new_base_tree(&mut stdout, &stdin)?;
    writeln!(stdout, "You're newly created tree:\n")?;
    tree.pretty_print(&mut stdout, 0)?;
    writeln!(stdout)?;

    write!(stdout, "Add {tree_name:?} to the config? ")?;
    stdout.flush()?;
    if yes_or_no(&mut stdout, &stdin, false)? {
        ctx.config.insert_tree(tree_name, tree);
    }
    writeln!(stdout)?;
    Ok(())
}

pub fn new_base_tree(o: &mut impl Write, i: &Stdin) -> Result<SpaceTree> {
    writeln!(o, "Which tree you want to insert?")?;
    writeln!(o, "1. Cmd")?;
    writeln!(o, "2. TmuxVSplit")?;
    writeln!(o, "3. TmuxHSplit")?;
    write!(o, ": ")?;
    o.flush()?;

    let mut buf = String::new();
    i.read_line(&mut buf)?;
    writeln!(o)?;

    let int = usize::from_str_radix(buf.trim(), 10).map_err(InteractiveError::InvalidInt)?;
    let tree = match int {
        1 => new_cmd_tree(o, i)?,
        2 => new_tmux_vsplit_tree(o, i)?,
        3 => new_tmux_hsplit_tree(o, i)?,
        _ => {
            return Err(DsError::InteractiveError(
                InteractiveError::UnknownTreeNumber(int),
            ));
        }
    };

    Ok(tree)
}

pub fn new_cmd_tree(o: &mut impl Write, i: &Stdin) -> Result<SpaceTree> {
    write!(o, "Type the command: ")?;
    o.flush()?;

    let mut buf = String::new();
    i.read_line(&mut buf)?;
    writeln!(o)?;
    Ok(SpaceTree::Cmd(buf.trim().to_string()))
}

pub fn new_tmux_vsplit_tree(o: &mut impl Write, i: &Stdin) -> Result<SpaceTree> {
    write!(o, "Make a left tree? ")?;
    let lhs = if yes_or_no(o, i, true)? {
        o.flush()?;
        Some(Box::new(new_base_tree(o, i)?))
    } else {
        None
    };

    write!(o, "Make a right tree? ")?;
    let rhs = if yes_or_no(o, i, true)? {
        o.flush()?;
        Some(Box::new(new_base_tree(o, i)?))
    } else {
        None
    };

    Ok(SpaceTree::TmuxVSplit { lhs, rhs })
}

pub fn new_tmux_hsplit_tree(o: &mut impl Write, i: &Stdin) -> Result<SpaceTree> {
    write!(o, "Make a top tree? ")?;
    let top = if yes_or_no(o, i, true)? {
        o.flush()?;
        Some(Box::new(new_base_tree(o, i)?))
    } else {
        None
    };

    write!(o, "Make a bottom tree? ")?;
    let bottom = if yes_or_no(o, i, true)? {
        o.flush()?;
        Some(Box::new(new_base_tree(o, i)?))
    } else {
        None
    };

    Ok(SpaceTree::TmuxHSplit { top, bottom })
}

pub fn yes_or_no(o: &mut impl Write, i: &Stdin, default_yes: bool) -> Result<bool> {
    if default_yes {
        write!(o, "[y]/n ")?;
    } else {
        write!(o, "y/[n] ")?;
    }
    o.flush()?;

    let mut buf = String::new();
    i.read_line(&mut buf)?;
    writeln!(o)?;

    if default_yes {
        match buf.trim() {
            "N" | "n" => Ok(false),
            _ => Ok(true),
        }
    } else {
        match buf.trim() {
            "Y" | "y" => Ok(true),
            _ => Ok(false),
        }
    }
}
