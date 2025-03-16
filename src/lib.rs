// TODO: be able to create Spaces with a name different than the directory it
// is in.
//
// TODO: make groups of command, like Spaces related commands, Trees related
// commands etc
//
// TODO: save the last used Space and put it in some file like the config so
// you aren't forced to put the name of the Space everywhere if there is
// already one stored.
//
// TODO: add a thing that checks if a new version is available and a config
// param to disable it. If a new version is available, print a warn when using
// the app.
//
// TODO: make the shell completions with `clap_complete`
use std::{
    env::{VarError, var},
    fmt::{Debug, Error as FmtError},
    fs::{File, create_dir_all, read_to_string},
    io::Write,
    panic::{self, AssertUnwindSafe},
    path::PathBuf,
};

use clap::{CommandFactory, FromArgMatches, Parser};
use ron::de::SpannedError;
use shadow_rs::shadow;
use thiserror::Error;
use tmux_interface::Error as TmuxError;

use crate::cmds::*;
use crate::config::{CmdParsingError, Config, SpaceTreeId};
use crate::database::DataBase;
use crate::new_tree::InteractiveError;

shadow!(build);
pub(crate) mod cmds;
pub mod config;
pub mod database;
pub mod repl;
pub mod utils;

const LONG_ABOUT: &str = "\
Devspace is a tool to save and retrieve your devlopment workspaces.";

pub type Result<T = (), E = DsError> = core::result::Result<T, E>;

/// DevSpace Error.
#[derive(Error, Debug)]
pub enum DsError {
    #[error(transparent)]
    VarError(#[from] VarError),
    #[error("IO: {0}")]
    IoError(#[from] std::io::Error),
    #[error("failed to parse a file, {0}.")]
    FileParsingError(#[from] SpannedError),
    #[error("failed to save the database: {0}")]
    DbSavingError(#[from] ron::Error),
    #[error("the space {0:?} was not found.")]
    SpaceNotFound(String),
    #[error("the space {0:?} already exists.")]
    SpaceAlreadyExists(String),
    #[error("TMUX: {0}")]
    TmuxError(#[from] TmuxError),
    #[error("space treee {:?} not found", .0.0)]
    SpaceTreeNotFound(SpaceTreeId),
    #[error("failed to parse command, {0}")]
    CmdParsingError(CmdParsingError),
    #[error("no space or tree to list.")]
    NothingToList,
    #[error(transparent)]
    FmtError(#[from] FmtError),
    #[error(transparent)]
    ClapError(#[from] clap::Error),
    #[error("failed to parse the command in the REPL.")]
    InvalidREPL,
    #[error("the directory {0:?} doesn't exists.")]
    DirDoesntExists(PathBuf),
    #[error(transparent)]
    InteractiveError(#[from] InteractiveError),
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = LONG_ABOUT)]
pub struct Cli {
    #[command(subcommand)]
    subcmds: Option<Command>,
    /// Overrides the directory where all devspace stuff is stored.
    ///
    /// Defaults to `$HOME/.devspace/`
    #[arg(long)]
    dir: Option<PathBuf>,
}

impl Cli {
    /// Gets the directory where to put the devspace stuff.
    pub fn dir(&self) -> Result<PathBuf> {
        // The order that will be checked for dir:
        // 1. the `--dir` argument
        // 2. the `DEVSPACE_DIR` variable
        // 3. the default `$HOME/.devspace/`

        // first the arg
        if let Some(d) = &self.dir {
            return Ok(d.to_path_buf());
        }

        // then the var
        if let Ok(dir) = var("DEVSPACE_DIR") {
            return Ok(dir.into());
        }

        // fallback to the default
        let mut default = PathBuf::from(var("HOME").expect("variable HOME not found wtf"));
        default.push(concat!(".", env!("CARGO_PKG_NAME"), "/"));
        Ok(default)
    }
}

#[derive(Parser, Debug)]
pub enum Command {
    /// Initializes a new development space.
    Init {
        /// Base path of the new Space.
        #[arg(default_value = ".")]
        path: PathBuf,
        /// What treee of Space it is, how to launch it.
        ///
        /// Defaults to the default set in the config.
        tree: Option<SpaceTreeId>,
    },
    /// Prints (to stdout) the working directory of a Space.
    Wdir {
        /// The space you want to goto.
        space: String,
    },
    /// Lists all the Spaces stored.
    ///
    /// Returns a non-zero exit code if there is no spaces stored.
    #[command(visible_alias = "ls")]
    ListSpaces,
    /// Lists all the Trees configured.
    ///
    /// Returns a non-zero exit code if there is no spaces stored.
    #[command(visible_alias = "lt")]
    ListTrees,
    /// Removes the Space with the given name.
    #[command(visible_alias = "rm-s")]
    RemoveSpace {
        /// Name of the Space to remove.
        space: String,
    },
    /// Go to the Space with the given name.
    ///
    /// If the Space has already been launched the Space isn't recreated.
    Go {
        /// Name of the Space to go to.
        space: String,
    },
    /// Edit a space config.
    Edit {
        /// Name of the Space to go to.
        space: String,
        /// The new working directory of the Space.
        #[arg(long, short)]
        wdir: Option<PathBuf>,
        /// The new tree of the Space.
        #[arg(long, short)]
        tree: Option<SpaceTreeId>,
    },
    /// Interactive tree creation.
    ///
    /// It overwrites the Tree if there is already a tree with this name.
    NewTree {
        /// Name of the Tree to be created.
        name: String,
    },
    /// Removes the Tree with the given name.
    #[command(visible_alias = "rm-t")]
    RemoveTree {
        /// Name of the Tree to remove.
        name: String,
    },
}

#[derive(Debug, Clone)]
pub struct Context {
    dir: PathBuf,
    db: DataBase,
    config: Config,
    /// Did we write back our buffered files?
    terminated: bool,
    /// The DataBase file buffer
    db_buf: String,
    /// The Config file buffer
    conf_buf: String,
}

impl Context {
    pub fn new(dir: PathBuf) -> Result<Context> {
        create_dir_all(&dir)?;
        let db_path = Context::db_file_path(&dir);
        let db_buf = if db_path.exists() {
            // file exists, read it and put it in buf
            read_to_string(Context::db_file_path(&dir))?
        } else {
            // file doesn't exist put the default in the buffer
            ron::ser::to_string_pretty(&DataBase::default(), utils::pretty_printer_config())?
        };

        let conf_path = Context::conf_file_path(&dir);
        let conf_buf = if conf_path.exists() {
            // file exists, read it and put it in buf
            read_to_string(Context::conf_file_path(&dir))?
        } else {
            // file doesn't exist put the default in the buffer
            ron::ser::to_string_pretty(&Config::default(), utils::pretty_printer_config())?
        };

        Ok(Context {
            dir,
            db: ron::from_str(&db_buf)?,
            config: ron::from_str(&conf_buf)?,
            terminated: false,
            db_buf,
            conf_buf,
        })
    }

    pub fn terminate(&mut self) -> Result {
        self.terminated = true;

        // write the db to the buf if we forgot to do se before.
        self.write_db_to_buf()?;

        // write back the database to file
        let mut db_file = File::create(Context::db_file_path(&self.dir))?;

        // here we are forced to clone because we later borrow self
        db_file.write_all(&self.db_buf.clone().into_bytes())?;

        // write the conf to the buf if we forgot to do se before.
        self.write_conf_to_buf()?;

        // write back the config to file
        let mut conf_file = File::create(Context::conf_file_path(&self.dir))?;
        // cannot move things because we implement Drop to check if we terminated.
        conf_file.write_all(&self.conf_buf.clone().into_bytes())?;

        Ok(())
    }

    pub(crate) fn write_db_to_buf(&mut self) -> Result {
        self.db_buf = ron::ser::to_string_pretty(&self.db, utils::pretty_printer_config())?;
        Ok(())
    }

    pub(crate) fn write_conf_to_buf(&mut self) -> Result {
        self.conf_buf = ron::ser::to_string_pretty(&self.config, utils::pretty_printer_config())?;
        Ok(())
    }

    pub(crate) fn db_file_path(dir: impl Into<PathBuf>) -> PathBuf {
        let mut dir = dir.into();
        dir.push("db.ron");
        dir
    }

    pub(crate) fn conf_file_path(dir: impl Into<PathBuf>) -> PathBuf {
        let mut dir = dir.into();
        dir.push("config.ron");
        dir
    }

    /// Returns the session name of the given `space`
    pub fn session_name(&self, space: &str) -> String {
        let mut sname = String::from("Space_");
        sname.push_str(space);
        sname
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        // if the context isn't terminated, then do it now.
        if !self.terminated {
            let res = panic::catch_unwind(AssertUnwindSafe(|| {
                #[cfg(debug_assertions)]
                eprintln!("INFO: Manually terminating the Context in the drop implementation.");
                self.terminate().unwrap();
            }));
            if res.is_err() {
                eprintln!(
                    "FATAL: The `drop` implementation of Context has panicked but luckily we catched it!"
                );
            }
        }
    }
}

pub fn run_command(args: Cli, ctx: &mut Context, repl: bool) -> Result {
    match args.subcmds {
        Some(Command::Init { path, tree }) => init::command(ctx, path, tree)?,
        Some(Command::Wdir { space }) => wdir::command(ctx, space)?,
        Some(Command::ListSpaces) => list_spaces::command(ctx)?,
        Some(Command::ListTrees) => list_trees::command(ctx)?,
        Some(Command::RemoveSpace { space }) => remove_space::command(ctx, space)?,
        Some(Command::Go { space }) => go::command(ctx, space)?,
        Some(Command::Edit { space, wdir, tree }) => edit::command(ctx, space, wdir, tree)?,
        Some(Command::NewTree { name }) => new_tree::command(ctx, name)?,
        Some(Command::RemoveTree { name }) => remove_tree::command(ctx, name)?,
        None if !repl => {
            repl::run()?;
        }
        None => {}
    }
    Ok(())
}

pub fn run() -> Result {
    let matches = Cli::command()
        .version(build::CLAP_LONG_VERSION)
        .get_matches();
    let args = Cli::from_arg_matches(&matches)?;

    let mut ctx = Context::new(args.dir()?)?;

    run_command(args, &mut ctx, false)?;

    ctx.terminate()?;

    Ok(())
}
