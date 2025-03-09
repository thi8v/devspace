use std::{collections::HashMap, fmt::Debug};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tmux_interface::{SendKeys, SplitWindow, TmuxCommands};

use crate::{DsError, Result, database::Space};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum SpaceTree {
    /// A command to run, the format is special.
    ///
    /// When executed `$(..)` will be parsed and its content can only be:
    /// - `Space.base` -> will be replaced by the base directory path of the
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
                            .target_pane("Space_devspace")
                            .key(parsed_cmd)
                            .into(),
                    )
                    .add_command(
                        SendKeys::new()
                            .target_pane("Space_devspace")
                            .key("C-m")
                            .into(),
                    );
                Ok(cmds)
            }
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
                        let replacement = match k.as_str() {
                            "Space.base" => space.base.clone(),
                            _ => return Err(CmdParsingError::UnknownPlaceholder(k)),
                        };
                        let replacement = replacement.to_str().unwrap();
                        res.push_str(replacement);
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
    space_trees: HashMap<SpaceTreeId, SpaceTree>,
}

impl Config {
    pub fn get_tree(&self, key: &SpaceTreeId) -> Result<&SpaceTree> {
        self.space_trees
            .get(key)
            .ok_or(DsError::SpaceTreeNotFound(key.clone()))
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            default_tree: "jump".into(),
            space_trees: HashMap::from([(
                "jump".into(),
                SpaceTree::Cmd(
                    "clear && echo 'Hello, welcome to the default devspace's tree'".to_string(),
                ),
            )]),
        }
    }
}
