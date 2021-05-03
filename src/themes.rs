use std::{path::{Path, PathBuf}, process::Command};
use std::collections::{hash_map, HashMap};

use crate::prelude::*;
use utils::tree_reader::{TreeReader, TreeReaderNode};

#[derive(Debug, Default)]
pub struct UnitDesc {
    pub values: HashMap<String, String>,
}

#[derive(Debug, Default)]
pub struct ThemeDesc {
    pub hooks: HookSet,
    pub units: HashMap<String, UnitDesc>,
}

#[derive(Debug)]
pub struct HookSet {
    pub preinstall: Hook,
    pub postinstall: Hook,
    pub preremove: Hook,
    pub postremove: Hook,
}

impl Default for HookSet {
    fn default() -> Self {
        HookSet {
            preinstall: Hook {
                name: String::from("Preinstall"),
                executables: Vec::new(),
            },
            postinstall: Hook {
                name: String::from("Postinstall"),
                executables: Vec::new(),
            },
            preremove: Hook {
                name: String::from("Preremove"),
                executables: Vec::new(),
            },
            postremove: Hook {
                name: String::from("Postremove"),
                executables: Vec::new(),
            },
        }
    }
}

#[derive(Debug)]
pub struct Hook {
    name: String,
    executables: Vec<(String, PathBuf)>,
}

impl Hook {
    pub fn run(&self) -> Result<(), Error> {
        for (name, executable) in &self.executables {
            let mut handle = Command::new(executable)
                .current_dir(executable.parent().unwrap())
                .spawn()
                .with_context(|| format!("Failed to start {} hook '{}'", self.name, name))?;

            let exit_status = handle.wait()?;
            if !exit_status.success() {
                if let Some(code) = exit_status.code() {
                    return Err(Error::Hook(format!("{} hook '{}' finished with exit code {}", self.name, name, code)));
                } else {
                    return Err(Error::Hook(format!("{} hook '{}' terminated by signal", self.name, name)));
                }
            }
        }

        Ok(())
    }
}

pub fn read_themes(dir: &Path) -> HashMap<String, ThemeDesc> {
    let mut themes = HashMap::<String, ThemeDesc>::new();

    let units_desc = &[
        TreeReaderNode::Literal(String::from("theme")),
        TreeReaderNode::Any,
        TreeReaderNode::Literal(String::from("unit")),
        TreeReaderNode::Any,
        TreeReaderNode::Any,
    ];
    for entry in TreeReader::new(dir, units_desc).get_file_entries_recursive() {
        assert_eq!(entry.captures.0.len(), 3);
        let mut captures = entry.captures.0;

        let value = std::fs::read_to_string(entry.path).unwrap();

        let value_name = captures.pop().unwrap();
        let unit_name = captures.pop().unwrap();
        let theme_name = captures.pop().unwrap();

        let theme = ensure_contains(&mut themes, theme_name);
        let unit = ensure_contains(&mut theme.units, unit_name);
        unit.values.insert(value_name, value);
    }

    let hooks_desc = &[
        TreeReaderNode::Literal(String::from("theme")),
        TreeReaderNode::Any,
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

        let theme = ensure_contains(&mut themes, theme_name);
        match hook_set_name.as_str() {
            "preinstall" => theme.hooks.preinstall.executables.push((hook_name, entry.path)),
            "postinstall" => theme.hooks.postinstall.executables.push((hook_name, entry.path)),
            "preremove" => theme.hooks.preremove.executables.push((hook_name, entry.path)),
            "postremove" => theme.hooks.postremove.executables.push((hook_name, entry.path)),
            _ => {}
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
