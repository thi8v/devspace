use std::{
    collections::{HashMap, hash_map::Iter},
    path::PathBuf,
};

use crate::{DsError, Result, config::SpaceTreeId};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DataBase {
    entries: HashMap<String, Space>,
}

impl DataBase {
    /// Retrieve the Space from its name.
    pub fn get_space(&self, space: &str) -> Result<&Space> {
        self.entries
            .get(space)
            .ok_or_else(|| DsError::SpaceNotFound(space.to_string()))
    }

    /// Inserts a new space with the given name (the key), if a space with the
    /// same name already exists it will be overwritten.
    pub fn insert(&mut self, key: String, space: Space) {
        self.entries.insert(key, space);
    }

    /// Iterator over the Spaces
    pub fn spaces_iter(&self) -> Iter<'_, String, Space> {
        self.entries.iter()
    }

    /// Is any Space contained?
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Remove the Space with the name provided as argument, does nothing if it
    /// doesn't exists.
    pub fn remove(&mut self, key: &str) {
        self.entries.remove(key);
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Space {
    // TODO: rename `base` to something like working directory it's more clear what it is.
    /// the base directory of
    pub base: PathBuf,
    /// the tree of Space, how to launch it.
    #[serde(rename = "tree")]
    pub tree: SpaceTreeId,
}

impl Space {
    pub fn new(base: PathBuf, tree: SpaceTreeId) -> Space {
        Space { base, tree }
    }
}
