use std::path::{Path, PathBuf};
use std::collections::{hash_map, HashMap};
use serde::Deserialize;

use crate::prelude::*;

#[derive(Debug, Default, Deserialize)]
pub struct UnitDesc {
    pub values: HashMap<String, String>,
}

impl UnitDesc {
    fn read_from_dir(dir: &Path) -> Result<Self, Error> {
        let values = utils::read_dir(dir, utils::ReadDirOptions::Files)?
            .map(|entry| {
                let key = entry.file_name;
                let value = std::fs::read_to_string(&entry.path)?;
                Ok((key, value))
            })
            .collect::<Result<HashMap<_, _>, Error>>()?;

        Ok(UnitDesc {
            values,
        })
    }

    fn add_value_from_file(&mut self, unit_name: &str, path: &Path) -> Result<(), Error> {
        let prefix = format!("unit-{}-", unit_name);

        let file_name = path.file_name().unwrap().to_str().unwrap();
        assert!(dbg!(file_name).starts_with(dbg!(&prefix)));

        let value_name = &file_name[prefix.len()..];
        let data = std::fs::read_to_string(path)?;

        self.values.insert(String::from(value_name), data);
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct ThemeDesc {
    pub hooks: (),
    #[serde(alias = "unit")]
    pub units: HashMap<String, UnitDesc>,
}

impl ThemeDesc {
    pub fn read_from_dir(dir: &Path) -> Result<Self, Error> {
        let units_dir = dir.join("units");

        let units = if std::fs::metadata(&units_dir).is_ok() {
            utils::read_dir(&units_dir, utils::ReadDirOptions::Directories)?
                .map(|entry| {
                    UnitDesc::read_from_dir(&entry.path)
                        .map(|desc| (entry.file_name, desc))
                })
                .collect::<Result<HashMap<String, UnitDesc>, Error>>()?
        } else {
            HashMap::new()
        };

        let mut desc = ThemeDesc {
            hooks: (),
            units,
        };
        desc.read_from_files(dir).unwrap();
        Ok(desc)
    }

    fn read_from_files(&mut self, dir: &Path) -> Result<(), Error> {
        let prefix = "unit-";

        let entries = utils::read_dir(dir, utils::ReadDirOptions::Files).unwrap()
            .filter(|entry| entry.file_name.starts_with(&prefix));
        for entry in entries {
            let dash = entry.file_name[prefix.len()..].find("-").unwrap();
            let path = &entry.path;
            let unit_name = &entry.file_name[prefix.len()..][..dash];
            match self.units.entry(String::from(unit_name)) {
                hash_map::Entry::Vacant(entry) => {
                    let mut unit = UnitDesc::default();
                    unit.add_value_from_file(unit_name, path).unwrap();
                    entry.insert(unit);
                }

                hash_map::Entry::Occupied(mut entry) => {
                    entry.get_mut().add_value_from_file(unit_name, path).unwrap();
                }
            }
        }

        Ok(())
    }
}
