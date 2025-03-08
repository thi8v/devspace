use std::{
    borrow::Cow,
    env::{VarError, var},
    path::{Path, PathBuf},
};

use clap::Parser;
use thiserror::Error;

const LONG_ABOUT: &str = "\
Devspace is a tool to save and retrieve your devlopment workspaces.";

pub type Result<T = (), E = DsError> = core::result::Result<T, E>;

/// DevSpace Error.
#[derive(Error, Debug)]
pub enum DsError {
    #[error(transparent)]
    VarError(#[from] VarError),
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = LONG_ABOUT)]
pub struct Cli {
    #[command(subcommand)]
    subcmds: SubCommands,
    /// Overrides the directory where all devspace stuff is stored.
    ///
    /// Defaults to `$HOME/.devspace/`
    #[arg(long)]
    dir: Option<PathBuf>,
}

impl Cli {
    /// Gets the directory where to put the devspace stuff.
    pub fn dir(&self) -> Cow<'_, Path> {
        match &self.dir {
            Some(d) => Cow::Borrowed(d),
            None => {
                let mut default = PathBuf::from(var("HOME").expect("variable HOME not found wtf"));
                default.push(concat!(".", env!("CARGO_PKG_NAME"), "/"));
                Cow::Owned(default)
            }
        }
    }
}

#[derive(Parser, Debug)]
pub enum SubCommands {
    /// Initializes a new development space.
    Init {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Change directory to the space base directory
    #[command(visible_alias = "cd")]
    JumpTo {
        /// The space you want to goto.
        space: String,
    },
}

pub fn run() -> Result {
    let args = Cli::parse();
    dbg!(&args);
    dbg!(args.dir());
    match args.subcmds {
        SubCommands::JumpTo { space } => jump2subcmd(space)?,
        scmd => todo!("{scmd:?}"),
    }

    Ok(())
}

pub(crate) fn jump2subcmd(space: String) -> Result {
    todo!("JUMP TO {space}");

    Ok(())
}
