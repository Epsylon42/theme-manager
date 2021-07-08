use std::path::{Path, PathBuf};
use std::collections::{hash_map, HashMap};

use crate::prelude::*;
use crate::hooks::{HookSet, HookLauncher};
use utils::tree_reader::{TreeReader, TreeReaderNode};

use regex::Regex;

#[derive(Debug, Default)]
pub struct UnitDesc {
    pub values: HashMap<String, String>,
}

#[derive(Debug, Default, serde::Deserialize)]
pub struct ThemeOptions {
    #[serde(default)]
    pub inherits: Option<String>
}

#[derive(Debug, Default)]
pub struct ThemeDesc {
    pub name: String,
    pub dir: PathBuf,
    pub hooks: HookSet,
    pub units: HashMap<String, UnitDesc>,
    pub options: ThemeOptions,
}

impl ThemeDesc {
    pub fn get_hook_launcher(&self) -> HookLauncher {
        HookLauncher::HookSet {
            theme_dir: &self.dir,
            theme_name: &self.name,
            hooks: &self.hooks,
        }
    }
}

pub fn read_from(dir: &Path) -> HashMap<String, ThemeDesc> {
    trace!("Reading themes from {:?}", dir);

    let mut themes = HashMap::<String, ThemeDesc>::new();

    let themes_desc = &[
        TreeReaderNode::Literal(String::from("theme")),
        TreeReaderNode::AnyDir,
    ];
    for mut entry in TreeReader::new(dir, themes_desc).get_dir_entries_recursive() {
        assert_eq!(entry.captures.0.len(), 1);
        let theme_name = entry.captures.0.pop().unwrap();
        trace!("Found theme '{}' in {:?}", theme_name, entry.path);

        let mut theme = ThemeDesc::default();
        theme.name = theme_name.clone();
        theme.dir = entry.path;

        let options_path = theme.dir.join("theme.toml");
        let options = match std::fs::read_to_string(&options_path) {
            Ok(options) => Some(options),

            Err(e) => {
                if options_path.exists() {
                    error!("Could not read options file ({}): skipping theme", e);
                    continue;
                }
                None
            }
        };

        if let Some(options) = options {
            let options = match toml::from_str(&options) {
                Ok(options) => options,

                Err(e) => {
                    error!("Could not parse options file ({})", e);
                    continue;
                }
            };

            theme.options = options;
        }

        *ensure_contains(&mut themes, theme_name) = theme;
    }

    read_units(dir, &mut themes);
    read_hooks(dir, &mut themes);

    themes
}

fn read_units(dir: &Path, themes: &mut HashMap<String, ThemeDesc>) {
    let unit_values_desc = &[
        TreeReaderNode::Literal(String::from("theme")),
        TreeReaderNode::AnyDir,
        TreeReaderNode::Literal(String::from("unit")),
        TreeReaderNode::Any,
        TreeReaderNode::Any,
    ];
    for entry in TreeReader::new(dir, unit_values_desc).get_file_entries_recursive() {
        assert_eq!(entry.captures.0.len(), 3);
        let mut captures = entry.captures.0;

        let value_name = captures.pop().unwrap();
        let unit_name = captures.pop().unwrap();
        let theme_name = captures.pop().unwrap();

        trace!("Found file with value '{}' for unit '{}' for theme '{}'", value_name, unit_name, theme_name);

        let theme = themes.get_mut(&theme_name)
            .expect("Found unit belonging to a nonexistent theme. This is probably a bug.");
        let unit = ensure_contains(&mut theme.units, unit_name);

        match read_value_file(&entry.path) {
            Ok(value) => {
                unit.values.insert(value_name, value);
            }
            Err(e) => {
                error!("Could not read value file {:?}: {}", entry.path, e);
                continue;
            }
        }
    }

    let units_compound_desc = &[
        TreeReaderNode::Literal(String::from("theme")),
        TreeReaderNode::AnyDir,
        TreeReaderNode::Literal(String::from("unit")),
        TreeReaderNode::Pattern(Regex::new("^(.*)\\.toml$").unwrap())
    ];
    for entry in TreeReader::new(dir, units_compound_desc).get_file_entries_recursive() {
        assert_eq!(entry.captures.0.len(), 2);
        let mut captures = entry.captures.0;

        let unit_name = captures.pop().unwrap();
        let theme_name = captures.pop().unwrap();

        trace!("Found compound file with values for unit '{}' for theme '{}'", unit_name, theme_name);

        let theme = themes.get_mut(&theme_name)
            .expect("Found unit belonging to a nonexistent theme. This is probably a bug.");
        let unit = ensure_contains(&mut theme.units, unit_name);

        match read_compound_file(&entry.path) {
            Ok(values) => {
                unit.values.extend(values);
            }
            Err(e) => {
                error!("Could not read compound file {:?}: {}", entry.path, e);
                continue;
            }
        }
    }
}

fn read_hooks(dir: &Path, themes: &mut HashMap<String, ThemeDesc>) {
    let hooks_desc = &[
        TreeReaderNode::Literal(String::from("theme")),
        TreeReaderNode::AnyDir,
        TreeReaderNode::Literal(String::from("hook")),
        TreeReaderNode::Any,
        TreeReaderNode::Any,
    ];
    for entry in TreeReader::new(dir, hooks_desc).get_file_entries_recursive() {
        assert_eq!(entry.captures.0.len(), 3);
        let mut captures = entry.captures.0;

        let hook_name = captures.pop().unwrap();
        let hook_set_name = captures.pop().unwrap();
        let theme_name = captures.pop().unwrap();

        trace!("Found {} hook '{}' for theme '{}'", hook_set_name, hook_name, theme_name);

        let theme = themes.get_mut(&theme_name)
            .expect("Found hook belonging to a nonexistent theme. This is probably a bug.");
        match hook_set_name.as_str() {
            "preinstall" => theme.hooks.preinstall.add(hook_name, entry.path),
            "postinstall" => theme.hooks.postinstall.add(hook_name, entry.path),
            "preremove" => theme.hooks.preremove.add(hook_name, entry.path),
            "postremove" => theme.hooks.postremove.add(hook_name, entry.path),
            _ => warn!("Hook set '{}' is invalid. Hook will be ignored", hook_set_name),
        }
    }
}

fn read_value_file(path: &Path) -> Result<String, Error> {
    Ok(std::fs::read_to_string(path)?)
}

fn read_compound_file(path: &Path) -> Result<HashMap<String, String>, Error> {
    let data = read_value_file(path)?;
    let values: HashMap<String, String> = toml::de::from_str::<HashMap<String, String>>(&data)
        .context("Format error")?;

    Ok(values)
}

fn ensure_contains<'a, T: Default>(map: &'a mut HashMap<String, T>, key: String) -> &'a mut T {
    match map.entry(key) {
        hash_map::Entry::Vacant(entry) => entry.insert(T::default()),
        hash_map::Entry::Occupied(entry) => entry.into_mut(),
    }
}
