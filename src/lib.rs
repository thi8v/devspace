// TODO: when there is more complex commands, add a repl like thing like you
// run `devspace` then you can run all the commands you want, use the clap thing yeah
//
// TODO: add a thing that checks if a new version is available and a config
// param to disable it. If a new version is available, print a warn when using
// the app.
use std::{
    env::{VarError, var},
    fmt::{Debug, Error as FmtError},
    fs::{File, create_dir_all, read_to_string},
    io::Write,
    path::PathBuf,
};

use clap::{CommandFactory, FromArgMatches, Parser};
use ron::de::SpannedError;
use thiserror::Error;
use tmux_interface::Error as TmuxError;

use crate::cmds::*;
use crate::config::{CmdParsingError, Config, SpaceTreeId};
use crate::database::DataBase;

pub(crate) mod cmds;
pub mod config;
pub mod database;
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
    #[error("failed to parse a database, {0}.")]
    DbParsingError(#[from] SpannedError),
    #[error("failed to save the database: {0}")]
    DbSavingError(#[from] ron::Error),
    #[error("the space {0:?} was not found.")]
    SpaceNotFound(String),
    #[error("the space {0:?} already exists.")]
    SpaceAlreadyExists(String),
    #[error("TMUX: {0}")]
    TmuxError(#[from] TmuxError),
    #[error("space treee {0:?} not found")]
    SpaceTreeNotFound(SpaceTreeId),
    #[error("failed to parse command, {0}")]
    CmdParsingError(CmdParsingError),
    #[error("no space or tree to list.")]
    NothingToList,
    #[error(transparent)]
    FmtError(#[from] FmtError),
    #[error(transparent)]
    ClapError(#[from] clap::Error),
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

// TODO: add a subcommand to change the Spaces settings, like:
// $ devspace change SPACE_NAME --tree TREE_NAME
// etc etc..
// TODO: add an interactive comand to create trees.
#[derive(Parser, Debug)]
pub enum SubCommands {
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
    Remove {
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

    pub fn terminate(mut self) -> Result {
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
        if !self.terminated {
            eprintln!("ERROR: You didn't terminate the context!")
        }
    }
}

pub fn run() -> Result {
    let matches = Cli::command()
        .version(env!("DEVSPACE_FULL_VERSION"))
        .get_matches();
    let args = Cli::from_arg_matches(&matches)?;

    let mut ctx = Context::new(args.dir()?)?;

    match args.subcmds {
        SubCommands::Init { path, tree } => init::command(&mut ctx, path, tree)?,
        SubCommands::Wdir { space } => wdir::command(&ctx, space)?,
        SubCommands::ListSpaces => list_spaces::command(&ctx)?,
        SubCommands::ListTrees => list_trees::command(&ctx)?,
        SubCommands::Remove { space } => remove::command(&mut ctx, space)?,
        SubCommands::Go { space } => go::command(&mut ctx, space)?,
    }

    ctx.terminate()?;

    Ok(())
}
