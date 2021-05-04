use std::{path::{Path, PathBuf}, process::Command};

use crate::prelude::*;
use utils::tree_reader::{TreeReader, TreeReaderNode};

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

pub enum HookLauncher<'a> {
    HookSet {
        theme_dir: &'a Path,
        theme_name: &'a str,
        hooks: &'a HookSet,
    },

    Empty,
}

impl<'a> HookLauncher<'a> {
    pub fn run_preinstall(&self) -> Result<(), Error> {
        match self {
            HookLauncher::HookSet { theme_dir, theme_name, hooks } => {
                hooks.preinstall.run(theme_dir, theme_name)
            }

            HookLauncher::Empty => Ok(()),
        }
    }

    pub fn run_postinstall(&self) -> Result<(), Error> {
        match self {
            HookLauncher::HookSet { theme_dir, theme_name, hooks } => {
                hooks.postinstall.run(theme_dir, theme_name)
            }

            HookLauncher::Empty => Ok(()),
        }
    }
}

#[derive(Debug)]
pub struct Hook {
    name: String,
    executables: Vec<(String, PathBuf)>,
}

impl Hook {
    pub fn run(&self, theme_dir: &Path, theme_name: &str) -> Result<(), Error> {
        for (name, executable) in &self.executables {
            let mut handle = Command::new(executable)
                .current_dir(executable.parent().unwrap())
                .arg(theme_dir)
                .arg(theme_name)
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

    pub fn add(&mut self, name: String, path: PathBuf) {
        self.executables.push((name, path));
    }
}

pub fn read_from(dir: &Path) -> HookSet {
    let mut hooks = HookSet::default();

    let hooks_desc = &[
        TreeReaderNode::Literal(String::from("hook")),
        TreeReaderNode::Any,
        TreeReaderNode::Any,
    ];
    for entry in TreeReader::new(dir, hooks_desc).get_file_entries_recursive() {
        assert_eq!(entry.captures.0.len(), 2);
        let mut captures = entry.captures.0;

        let hook_name = captures.pop().unwrap();
        let hook_set_name = captures.pop().unwrap();
        match hook_set_name.as_str() {
            "preinstall" => hooks.preinstall.add(hook_name, entry.path),
            "postinstall" => hooks.postinstall.add(hook_name, entry.path),
            "preremove" => hooks.preremove.add(hook_name, entry.path),
            "postremove" => hooks.postremove.add(hook_name, entry.path),
            _ => {}
        }
    }

    hooks
}
