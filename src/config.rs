use std::{collections::HashMap, fmt::Debug, io::Write};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tmux_interface::{SelectPane, SendKeys, SplitWindow, TmuxCommands};

use crate::{DsError, Result, database::Space};

/// A tree, represents what the environment will look like.
//  /!\ If a tree is create update the `new-tree` command.
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
        lhs: Option<Box<SpaceTree>>,
        rhs: Option<Box<SpaceTree>>,
    },
    /// Launch tmux if not already in a Tmux session and split the pane in two
    /// horizontally. A Space Tree will be applied to the top and one to the
    /// bottom.
    TmuxHSplit {
        top: Option<Box<SpaceTree>>,
        bottom: Option<Box<SpaceTree>>,
    },
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
            Self::TmuxVSplit { lhs, rhs } => {
                let mut cmds = TmuxCommands::new();

                // push the split first
                cmds.push(SplitWindow::new().horizontal().target_window(space_name));

                // then push the left
                if let Some(lhs) = lhs {
                    cmds.push(SelectPane::new().left());
                    let lhs = lhs.build(space, space_name)?;
                    cmds.push_cmds(lhs);
                }

                // finally push the right
                if let Some(rhs) = rhs {
                    cmds.push(SelectPane::new().right());
                    let rhs = rhs.build(space, space_name)?;
                    cmds.push_cmds(rhs);
                }

                Ok(cmds)
            }
            Self::TmuxHSplit { top, bottom } => {
                let mut cmds = TmuxCommands::new();

                // push the split first
                cmds.push(SplitWindow::new().vertical().target_window(space_name));

                // then push the top
                if let Some(top) = top {
                    cmds.push(SelectPane::new().up());
                    let top = top.build(space, space_name)?;
                    cmds.push_cmds(top);
                }

                // finally push the bottom
                if let Some(bottom) = bottom {
                    cmds.push(SelectPane::new().down());
                    let bottom = bottom.build(space, space_name)?;
                    cmds.push_cmds(bottom);
                }

                Ok(cmds)
            }
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
                if let Some(lhs) = lhs {
                    lhs.pretty_print(w, indent + Self::PRINT_INDENT)?;
                } else {
                    writeln!(w, "None")?;
                }

                write!(w, "{:indent$}  | rhs: ", "")?;
                if let Some(rhs) = rhs {
                    rhs.pretty_print(w, indent + Self::PRINT_INDENT)?;
                } else {
                    writeln!(w, "None")?;
                }
            }
            Self::TmuxHSplit { top, bottom } => {
                writeln!(w, "TmuxHSplit:")?;

                write!(w, "{:indent$}  | top: ", "")?;
                if let Some(top) = top {
                    top.pretty_print(w, indent + Self::PRINT_INDENT)?;
                } else {
                    writeln!(w, "None")?;
                }
                write!(w, "{:indent$}  | bottom: ", "")?;
                if let Some(bottom) = bottom {
                    bottom.pretty_print(w, indent + Self::PRINT_INDENT)?;
                } else {
                    writeln!(w, "None")?;
                }
            }
            Self::Cmd(cmd) => {
                writeln!(w, "Cmd({cmd:?})")?;
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
        let mut chars = cmd.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                '{' => {
                    if chars.peek() == Some(&'{') {
                        chars.next(); // Consume second '{'
                        res.push('{');
                    } else {
                        key = Some(String::new());
                    }
                }
                '}' => {
                    if chars.peek() == Some(&'}') {
                        chars.next(); // Consume second '}'
                        res.push('}');
                    } else if let Some(k) = key.take() {
                        let replacement: String = match k.as_str() {
                            "Space.wdir" => space.wdir.clone().to_string_lossy().into_owned(),
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

    pub fn insert_tree(&mut self, tree_name: String, tree: SpaceTree) {
        self.trees.insert(SpaceTreeId(tree_name), tree);
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
