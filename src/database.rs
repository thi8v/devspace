use std::{
    collections::{HashMap, hash_map::Iter},
    fs::File,
    io::{Read, Write},
    path::PathBuf,
};

use crate::{DsError, Result};

use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DataBase {
    entries: HashMap<String, Space>,
}

impl DataBase {
    /// Reads a file that contains the database and returns it deserialized.
    pub fn from_file(mut file: File) -> Result<DataBase> {
        let mut buf = vec![];
        file.read_to_end(&mut buf)?;
        let buf_str = String::from_utf8_lossy(&buf);

        let db = ron::from_str(&buf_str)?;
        Ok(db)
    }

    /// Retrieve the Space from its name.
    pub fn get_space(&self, space: &str) -> Result<&Space> {
        Ok(self
            .entries
            .get(space)
            .ok_or_else(|| DsError::SpaceNotFound(space.to_string()))?)
    }

    /// Saves the database into the provided file.
    pub fn save(&self, mut file: File) -> Result {
        // TODO: can we put it in drop? i think no because this function can fail
        let ron_str = ron::ser::to_string_pretty(self, PrettyConfig::default())?;

        let buf = ron_str.as_bytes();

        file.write(buf)?;
        Ok(())
    }

    /// Inserts a new space with the given name (the key), if a space with the
    /// same name already exists it will be overwritten.
    pub fn insert(&mut self, key: String, space: Space) {
        self.entries.insert(key.clone(), space);
    }

    pub fn spaces_iter(&self) -> Iter<'_, String, Space> {
        self.entries.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn remove(&mut self, key: &str) {
        self.entries.remove(key);
    }
}

impl Default for DataBase {
    fn default() -> Self {
        DataBase {
            entries: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Space {
    /// the base directory of
    pub base: PathBuf,
}

impl Space {
    pub fn new(base: PathBuf) -> Space {
        Space { base }
    }
}
