use std::{
    fs::File,
    io::{Read, Write},
};

use ron::ser::PrettyConfig;
use serde::{Serialize, de::DeserializeOwned};

use crate::Result;

pub fn pretty_printer_config() -> PrettyConfig {
    let mut conf = PrettyConfig::default();
    conf.struct_names = true;
    conf.separate_tuple_members = true;
    conf.enumerate_arrays = true;
    conf
}

/// Reads a file that contains our RON formatted data and returns it deserialized.
pub fn from_ron_file<T>(mut file: File) -> Result<T>
where
    T: DeserializeOwned,
{
    let mut buf = vec![];
    file.read_to_end(&mut buf)?;
    let buf_str = String::from_utf8_lossy(&buf);

    let db = ron::from_str(&buf_str)?;
    Ok(db)
}

/// Saves the data passed as argument in the given file.
pub fn save_ron_file<T>(data: &T, mut file: File) -> Result
where
    T: ?Sized + Serialize,
{
    let ron_str = ron::ser::to_string_pretty(data, pretty_printer_config())?;

    let buf = ron_str.as_bytes();

    file.write_all(buf)?;
    Ok(())
}
