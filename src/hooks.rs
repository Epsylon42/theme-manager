use std::{
    path::{Path, PathBuf},
    process::Command,
};

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
                global: false,
                name: String::from("Preinstall"),
                executables: Vec::new(),
            },
            postinstall: Hook {
                global: false,
                name: String::from("Postinstall"),
                executables: Vec::new(),
            },
            preremove: Hook {
                global: false,
                name: String::from("Preremove"),
                executables: Vec::new(),
            },
            postremove: Hook {
                global: false,
                name: String::from("Postremove"),
                executables: Vec::new(),
            },
        }
    }
}

impl HookSet {
    pub fn global() -> Self {
        let mut set = HookSet::default();
        for hook in [
            &mut set.preinstall,
            &mut set.postinstall,
            &mut set.preremove,
            &mut set.postremove,
        ] {
            hook.global = true;
        }

        set
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
            HookLauncher::HookSet {
                theme_dir,
                theme_name,
                hooks,
            } => hooks.preinstall.run(theme_dir, theme_name),

            HookLauncher::Empty => Ok(()),
        }
    }

    pub fn run_postinstall(&self) -> Result<(), Error> {
        match self {
            HookLauncher::HookSet {
                theme_dir,
                theme_name,
                hooks,
            } => hooks.postinstall.run(theme_dir, theme_name),

            HookLauncher::Empty => Ok(()),
        }
    }
}

#[derive(Debug)]
pub struct Hook {
    global: bool,
    name: String,
    executables: Vec<(String, PathBuf)>,
}

impl Hook {
    fn run(&self, theme_dir: &Path, theme_name: &str) -> Result<(), Error> {
        if self.global {
            trace!("Running global {} hook for theme {}", self.name, theme_name);
        } else {
            trace!("Running {} hook for theme {}", self.name, theme_name);
        }

        for (name, executable) in &self.executables {
            trace!("Running executable '{}' at {:?}", name, executable);

            let mut handle = Command::new(executable)
                .current_dir(
                    executable
                        .parent()
                        .expect("Hook path does not have a parent. This is probably a bug"),
                )
                .arg(theme_dir)
                .arg(theme_name)
                .spawn()
                .with_context(|| format!("Failed to start {} hook '{}'", self.name, name))?;

            let exit_status = handle.wait()?;
            if !exit_status.success() {
                return Err(Error::Hook {
                    name: self.name.clone(),
                    executable: name.clone(),
                    cause: if let Some(code) = exit_status.code() {
                        format!("finished with exit code {}", code)
                    } else {
                        String::from("terminated by signal")
                    },
                });
            }
        }

        Ok(())
    }

    pub fn add(&mut self, name: String, path: PathBuf) {
        self.executables.push((name, path));
    }
}

pub fn read_from(dir: &Path) -> HookSet {
    trace!("Reading global hooks from {:?}", dir);

    let mut hooks = HookSet::global();

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

        trace!("Found {} hook '{}'", hook_set_name, hook_name);

        match hook_set_name.as_str() {
            "preinstall" => hooks.preinstall.add(hook_name, entry.path),
            "postinstall" => hooks.postinstall.add(hook_name, entry.path),
            "preremove" => hooks.preremove.add(hook_name, entry.path),
            "postremove" => hooks.postremove.add(hook_name, entry.path),
            _ => warn!(
                "Hook set '{}' is invalid. Hook will be ignored",
                hook_set_name
            ),
        }
    }

    hooks
}
