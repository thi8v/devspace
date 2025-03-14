use std::{collections::HashMap, fmt::Debug, io::Write};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tmux_interface::{SendKeys, SplitWindow, TmuxCommands};

use crate::{DsError, Result, database::Space};

// TODO: add support for other things than Tmux, create a `Tmux(Tree)` Tree
// that is necessary to use `Tmux*` trees. Yeah but what?
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum SpaceTree {
    /// A command to run, the format is special.
    ///
    /// When executed `$(..)` will be parsed and its content can only be:
    /// - `Space.wdir` -> will be replaced by the working directory path of the
    ///   Space that is runned.
    Cmd(String),
    /// Launch tmux if not already in a Tmux session and split the pane in two
    /// vertically. A Space Tree will be applied to the left and one to the
    /// right.
    TmuxVSplit {
        lhs: Box<SpaceTree>,
        rhs: Box<SpaceTree>,
    },
    /// Launch tmux if not already in a Tmux session and split the pane in two
    /// horizontally. A Space Tree will be applied to the top and one to the
    /// bottom.
    TmuxHSplit {
        top: Box<SpaceTree>,
        bottom: Box<SpaceTree>,
    },
    // TODO: rename this name sucks or make the `lhs`, `rhs`, `top`, `bottom`
    // fields Options so no need for this and it avoids allocation.
    /// The default thing that is runned in a Tmux Pane.
    TmuxDefault,
}

impl SpaceTree {
    pub fn build<'a>(&self, space: &Space, space_name: &'a str) -> Result<TmuxCommands<'a>> {
        match self {
            Self::Cmd(cmd) => {
                let parsed_cmd = cmd_placeholders(cmd, space)?;
                let cmds = TmuxCommands::new()
                    .add_command(
                        SendKeys::new()
                            .target_pane(space_name)
                            .key(parsed_cmd)
                            .into(),
                    )
                    .add_command(SendKeys::new().target_pane(space_name).key("C-m").into());
                Ok(cmds)
            }
            // TODO: fix the splits it doesn't work like it should be working.
            Self::TmuxVSplit { lhs, rhs } => {
                let mut cmds = TmuxCommands::new();

                // push the lhs first
                let lhs = lhs.build(space, space_name)?;
                cmds.push_cmds(lhs);

                // push the split
                cmds.push(SplitWindow::new().horizontal().target_window(space_name));

                // finally push the rhs
                let rhs = rhs.build(space, space_name)?;
                cmds.push_cmds(rhs);

                Ok(cmds)
            }
            Self::TmuxHSplit { top, bottom } => {
                let mut cmds = TmuxCommands::new();

                // push the top first
                let top = top.build(space, space_name)?;
                cmds.push_cmds(top);

                // push the split
                cmds.push(SplitWindow::new().vertical().target_window(space_name));

                // finally push the bottom
                let bottom = bottom.build(space, space_name)?;
                cmds.push_cmds(bottom);

                Ok(cmds)
            }
            Self::TmuxDefault => Ok(TmuxCommands::new()),
        }
    }

    pub const PRINT_INDENT: usize = 2;

    /// Prints the Tree with a Pretty AST like syntax.
    ///
    /// Do not flush the Writer, you may need to `flush` it.
    pub fn pretty_print(&self, w: &mut impl Write, indent: usize) -> Result {
        match self {
            Self::TmuxVSplit { lhs, rhs } => {
                writeln!(w, "TmuxVSplit:")?;
                write!(w, "{:indent$}  | lhs: ", "")?;
                lhs.pretty_print(w, indent + Self::PRINT_INDENT)?;
                write!(w, "{:indent$}  | rhs: ", "")?;
                rhs.pretty_print(w, indent + Self::PRINT_INDENT)?;
            }
            Self::TmuxHSplit { top, bottom } => {
                writeln!(w, "TmuxHSplit:")?;
                write!(w, "{:indent$}  | top: ", "")?;
                top.pretty_print(w, indent + Self::PRINT_INDENT)?;
                write!(w, "{:indent$}  | bottom: ", "")?;
                bottom.pretty_print(w, indent + Self::PRINT_INDENT)?;
            }
            Self::Cmd(cmd) => {
                writeln!(w, "Cmd({cmd:?})")?;
            }
            Self::TmuxDefault => {
                writeln!(w, "TmuxDefault")?;
            }
        }
        Ok(())
    }
}

/// Cmd Parsing Error.
#[derive(Error, Debug)]
pub enum CmdParsingError {
    #[error("unknown placeholder {0}.")]
    UnknownPlaceholder(String),
    #[error("a '}}' was found but no matching '{{' has been found.")]
    ClosingBracketNoOpening,
    #[error("a '{{' was found but no matching '}}' has been found.")]
    OpeningBracketNoClosing,
}

fn cmd_placeholders(cmd: &str, space: &Space) -> Result<String> {
    fn cmd_placeholders_inner(cmd: &str, space: &Space) -> Result<String, CmdParsingError> {
        let mut res = String::new();

        let mut key = None;

        // TODO: support double brackets and do nothing just put one of the brackets to still be able to use { and }
        for ch in cmd.chars() {
            match ch {
                '{' => key = Some(String::new()),
                '}' => {
                    if let Some(k) = key.take() {
                        let replacement: String = match k.as_str() {
                            "Space.wdir" => {
                                let s = space.wdir.clone().to_string_lossy().into_owned();
                                s
                            }
                            _ => return Err(CmdParsingError::UnknownPlaceholder(k)),
                        };
                        res.push_str(&replacement);
                    } else {
                        return Err(CmdParsingError::ClosingBracketNoOpening);
                    }
                }
                _ => {
                    if let Some(ref mut k) = key {
                        k.push(ch);
                    } else {
                        res.push(ch);
                    }
                }
            }
        }
        if let Some(k) = key.take() {
            if !k.is_empty() {
                return Err(CmdParsingError::OpeningBracketNoClosing);
            }
        }

        Ok(res)
    }

    cmd_placeholders_inner(cmd, space).map_err(DsError::CmdParsingError)
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct SpaceTreeId(pub(crate) String);

impl Debug for SpaceTreeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Id({:?})", self.0)
    }
}

impl From<&str> for SpaceTreeId {
    fn from(value: &str) -> Self {
        SpaceTreeId(String::from(value))
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub default_tree: SpaceTreeId,
    pub(crate) trees: HashMap<SpaceTreeId, SpaceTree>,
}

impl Config {
    pub fn get_tree(&self, key: &SpaceTreeId) -> Result<&SpaceTree> {
        self.trees
            .get(key)
            .ok_or(DsError::SpaceTreeNotFound(key.clone()))
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            default_tree: "jump".into(),
            trees: HashMap::from([(
                "jump".into(),
                SpaceTree::Cmd(
                    "clear && echo 'Hello, welcome to the default devspace's tree'".to_string(),
                ),
            )]),
        }
    }
}
