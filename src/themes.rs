use std::path::{Path, PathBuf};
use std::collections::{hash_map, HashMap};

use crate::prelude::*;
use crate::hooks::{HookSet, HookLauncher};
use utils::tree_reader::{TreeReader, TreeReaderNode};

#[derive(Debug, Default)]
pub struct UnitDesc {
    pub values: HashMap<String, String>,
}

#[derive(Debug, Default)]
pub struct ThemeDesc {
    pub name: String,
    pub dir: PathBuf,
    pub hooks: HookSet,
    pub units: HashMap<String, UnitDesc>,
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

        let theme = ensure_contains(&mut themes, theme_name.clone());
        theme.name = theme_name;
        theme.dir = entry.path;
    }

    let units_desc = &[
        TreeReaderNode::Literal(String::from("theme")),
        TreeReaderNode::AnyDir,
        TreeReaderNode::Literal(String::from("unit")),
        TreeReaderNode::Any,
        TreeReaderNode::Any,
    ];
    for entry in TreeReader::new(dir, units_desc).get_file_entries_recursive() {
        assert_eq!(entry.captures.0.len(), 3);
        let mut captures = entry.captures.0;

        let value_name = captures.pop().unwrap();
        let unit_name = captures.pop().unwrap();
        let theme_name = captures.pop().unwrap();

        trace!("Found file with value '{}' for unit '{}' for theme '{}'", value_name, unit_name, theme_name);

        match std::fs::read_to_string(&entry.path) {
            Ok(value) =>  {
                let theme = themes.get_mut(&theme_name)
                    .expect("Found unit belonging to a nonexistent theme. This is probably a bug.");
                let unit = ensure_contains(&mut theme.units, unit_name);
                unit.values.insert(value_name, value);
            }
            Err(e) => {
                error!("Could not read {:?}: {}", entry.path, e);
                continue;
            }
        }
    }

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

    themes
}

fn ensure_contains<'a, T: Default>(map: &'a mut HashMap<String, T>, key: String) -> &'a mut T {
    match map.entry(key) {
        hash_map::Entry::Vacant(entry) => entry.insert(T::default()),
        hash_map::Entry::Occupied(entry) => entry.into_mut(),
    }
}
