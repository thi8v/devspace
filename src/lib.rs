use std::{
    borrow::Cow,
    env::{VarError, var},
    fmt::Debug,
    fs::{File, OpenOptions, canonicalize, create_dir_all},
    path::{Path, PathBuf},
};

use clap::Parser;
use database::{DataBase, Space};
use ron::de::SpannedError;
use thiserror::Error;

pub mod database;

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
        /// Base path of the new Space.
        #[arg(default_value = ".")]
        path: PathBuf,
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
}

#[derive(Debug, Clone)]
pub struct Context {
    dir: PathBuf,
    db: DataBase,
}

impl Context {
    pub fn new(dir: PathBuf) -> Result<Context> {
        create_dir_all(&dir)?;
        let db_path = Context::db_file_path(dir.clone());

        let db_file = OpenOptions::new()
            .write(true)
            .read(true)
            .create(true) // TODO: instead of use this create fn, make a custom
            // one where it puts a basic database containing DataBase::default() serialized
            .open(db_path)?;

        Ok(Context {
            dir,
            db: DataBase::from_file(db_file)?,
        })
    }

    pub fn db_file_path(mut db_path: PathBuf) -> PathBuf {
        db_path.push("db.ron");
        db_path
    }

    pub fn db_file(&self) -> PathBuf {
        Context::db_file_path(self.dir.clone())
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

    pub(crate) fn init_subcmd(&mut self, path: PathBuf) -> Result {
        let abs = canonicalize(path)?;

        // TODO: remove the unwrap
        let dir_name = abs.file_name().unwrap().to_string_lossy();

        if self.db.get_space(&dir_name).is_ok() {
            return Err(DsError::SpaceAlreadyExists(dir_name.into_owned()));
        }

        self.db.insert(dir_name.into_owned(), Space::new(abs));

        let db_file = File::create(self.db_file())?;

        self.db.save(db_file)?;

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
        let width = spaces.iter().map(|(s, _)| s.len()).max().unwrap() + 5;

        for (name, space) in spaces {
            println!("{name:width$} {}", space.base.to_str().unwrap());
        }
    }

    pub(crate) fn remove_subcmd(&mut self, space: String) -> Result {
        if self.db.get_space(&space).is_err() {
            return Err(DsError::SpaceAlreadyExists(space));
        }

        self.db.remove(&space);

        let db_file = File::create(self.db_file())?;

        self.db.save(db_file)?;

        Ok(())
    }
}

pub fn run() -> Result {
    let args = Cli::parse();

    let mut ctx = Context::new(args.dir().into_owned())?;

    match args.subcmds {
        SubCommands::Base { space } => ctx.base_subcmd(space)?,
        SubCommands::Init { path } => ctx.init_subcmd(path)?,
        SubCommands::ListSpaces => ctx.list_spaces_subcmd(),
        SubCommands::Remove { space } => ctx.remove_subcmd(space)?,
    }

    Ok(())
}
