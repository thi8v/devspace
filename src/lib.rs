use std::{
    borrow::Cow,
    env::{VarError, var},
    fmt::Debug,
    fs::{File, canonicalize, create_dir_all, read_to_string},
    io::Write,
    path::{Path, PathBuf},
};

use clap::Parser;
use config::{CmdParsingError, Config, SpaceTreeId};
use database::{DataBase, Space};
use ron::de::SpannedError;
use thiserror::Error;
use tmux_interface::{
    AttachSession, Error as TmuxError, HasSession, NewSession, StdIO, Tmux, TmuxCommands,
};

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
        // TODO: query for the `DEVSPACE_DIR` variable, it could contain the directory.
        // The order that will be checked for dir:
        // 1. the `--dir` argument
        // 2. the `DEVSPACE_DIR` variable
        // 3. the default `$HOME/.devspace/`

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

// TODO: add a subcommand to change the Spaces settings, like:
// $ devspace change SPACE_NAME --tree TREE_NAME
// etc etc..
// TODO: add a `list-trees` subcommand
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
    /// Prints (to stdout) the base directory of a Space.
    Base {
        /// The space you want to goto.
        space: String,
    },
    /// Lists all the Spaces stored.
    #[command(visible_alias = "ls")]
    ListSpaces,
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
        let db_path = Context::db_file_path(dir.clone());
        let db_buf;

        if db_path.exists() {
            // file exists, read it and put it in buf
            db_buf = read_to_string(Context::db_file_path(dir.clone()))?;
        } else {
            // file doesn't exist put the default in the buffer
            db_buf =
                ron::ser::to_string_pretty(&DataBase::default(), utils::pretty_printer_config())?;
        }

        let conf_path = Context::conf_file_path(dir.clone());
        let conf_buf;

        if conf_path.exists() {
            // file exists, read it and put it in buf
            conf_buf = read_to_string(Context::conf_file_path(dir.clone()))?;
        } else {
            // file doesn't exist put the default in the buffer
            conf_buf =
                ron::ser::to_string_pretty(&Config::default(), utils::pretty_printer_config())?;
        }

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

        // write back the database to file
        let mut db_file = File::create(Context::db_file_path(self.dir.clone()))?;
        db_file.write_all(&self.db_buf.into_bytes())?;

        // write back the config to file
        let mut conf_file = File::create(Context::conf_file_path(self.dir.clone()))?;
        conf_file.write_all(&self.conf_buf.into_bytes())?;

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

    pub(crate) fn db_file_path(mut dir: PathBuf) -> PathBuf {
        dir.push("db.ron");
        dir
    }

    pub(crate) fn conf_file_path(mut dir: PathBuf) -> PathBuf {
        dir.push("config.ron");
        dir
    }

    pub(crate) fn base_subcmd(&self, space_name: String) -> Result {
        let space = self.db.get_space(&space_name)?;

        println!(
            "{}",
            space
                .base
                .clone()
                .into_os_string()
                .into_string()
                .unwrap_or_default()
        );
        Ok(())
    }

    pub(crate) fn init_subcmd(&mut self, path: PathBuf, tree: Option<SpaceTreeId>) -> Result {
        let abs = canonicalize(path)?;

        // TODO: remove the unwrap
        let dir_name = abs.file_name().unwrap().to_string_lossy();

        if self.db.get_space(&dir_name).is_ok() {
            return Err(DsError::SpaceAlreadyExists(dir_name.into_owned()));
        }

        self.db.insert(
            dir_name.into_owned(),
            Space::new(abs, tree.unwrap_or(self.config.default_tree.clone())),
        );

        self.write_db_to_buf()?;

        Ok(())
    }

    pub(crate) fn list_spaces_subcmd(&self) {
        if self.db.is_empty() {
            // TODO: return an error if there is no Spaces?
            return;
        }

        let mut spaces = self.db.spaces_iter().collect::<Vec<_>>();
        spaces.sort_by(|a, b| a.0.cmp(b.0));

        // can safely unwrap because we know there is at least one value.
        let name_width = spaces.iter().map(|(s, _)| s.len()).max().unwrap().max(8);
        let path_width = spaces
            .iter()
            .map(|(_, s)| s.base.to_str().unwrap().len())
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
                space.base.to_str().unwrap(),
                space.tree.0
            );
        }
    }

    pub(crate) fn remove_subcmd(&mut self, space: String) -> Result {
        if self.db.get_space(&space).is_err() {
            return Err(DsError::SpaceAlreadyExists(space));
        }

        self.db.remove(&space);

        self.write_db_to_buf()?;

        Ok(())
    }

    pub(crate) fn go_subcmd(&mut self, space_name: String) -> Result {
        let session_name = self.session_name(&space_name);
        let space = self.db.get_space(&space_name)?;

        let session_exists = Tmux::with_command(HasSession::new().target_session(&session_name))
            .output()?
            .success();

        // the session already exists, don't create another one just attach to it.
        if session_exists {
            let _ = Tmux::with_command(AttachSession::new().target_session(&session_name))
                .stdin(Some(StdIO::Inherit))
                .stdout(Some(StdIO::Inherit))
                .stderr(Some(StdIO::Inherit))
                .output()?;

            return Ok(());
        }

        let mut cmds = TmuxCommands::new().add_command(
            NewSession::new()
                .attach()
                .session_name(&session_name)
                .start_directory(space.base.to_str().unwrap())
                .into(),
        );

        let tree = self.config.get_tree(&space.tree)?;
        let session_name = &self.session_name(&space_name);
        let built_treee = tree.build(space, session_name)?;

        cmds.push_cmds(built_treee);

        let _ = Tmux::with_commands(cmds)
            .stdin(Some(StdIO::Inherit))
            .stdout(Some(StdIO::Inherit))
            .stderr(Some(StdIO::Inherit))
            .output()?;

        Ok(())
    }

    /// Returns the session name of the given `space`
    pub fn session_name(&self, space: &str) -> String {
        let mut sname = String::from("Space_");
        sname.push_str(space);
        sname
    }
}

pub fn run() -> Result {
    let args = Cli::parse();

    let mut ctx = Context::new(args.dir().into_owned())?;

    match args.subcmds {
        SubCommands::Base { space } => ctx.base_subcmd(space)?,
        SubCommands::Init { path, tree } => ctx.init_subcmd(path, tree)?,
        SubCommands::ListSpaces => ctx.list_spaces_subcmd(),
        SubCommands::Remove { space } => ctx.remove_subcmd(space)?,
        SubCommands::Go { space } => ctx.go_subcmd(space)?,
    }

    ctx.terminate()?;

    Ok(())
}
