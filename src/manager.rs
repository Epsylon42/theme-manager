use std::{collections::HashMap, path::{Path, PathBuf}};

use crate::prelude::*;
use crate::install::{self, InstallDesc};
use crate::themes::{self, ThemeDesc};
use crate::hooks::{self, HookSet};

#[derive(Debug)]
pub struct ThemeManager {
    dir: PathBuf,
    install: InstallDesc,
    themes: HashMap<String, ThemeDesc>,
    global_hooks: HookSet,
}

impl ThemeManager {
    pub fn read_from_dir(dir: &Path) -> Result<Self, Error> {
        Ok(ThemeManager {
            dir: dir.to_owned(),
            install: install::read_from(&dir.join("install"))
                .context("Could not read install directory")?,
            themes: themes::read_from(dir),
            global_hooks: hooks::read_from(dir),
        })
    }

    pub fn install_theme(&self, theme: &str) {
        let mut theme_chain = Vec::new();
        let mut theme = self.themes.get(theme).expect("Theme does not exist");
        while let Some(ref inherits) = theme.options.inherits {
            let next = self.themes.get(inherits).expect("Inherited theme does not exist");
            theme_chain.push(theme);
            theme = next;
        }
        theme_chain.push(theme);
        if let Some(default) = self.themes.get("default") {
            if default as *const _ != (*theme_chain.last().unwrap()) as *const _ {
                theme_chain.push(default);
            }
        }
        theme_chain.reverse();

        self.install.install(&theme_chain, hooks::HookLauncher::HookSet {
            theme_dir: &theme.dir,
            theme_name: &theme.name,
            hooks: &self.global_hooks,
        }).unwrap();
    }

    pub fn install_empty(&self) {
        self.install.install_empty(hooks::HookLauncher::HookSet {
            theme_dir: &self.dir,
            theme_name: "empty",
            hooks: &self.global_hooks,
        }).unwrap();
    }

    pub fn update(&self) {
        let installed_theme_file = self.dir.join(".cache/installed");
        if installed_theme_file.exists() {
            let theme_name = std::fs::read_to_string(installed_theme_file).expect("Could not read installed theme file");
            self.install_theme(&theme_name);
        } else {
            eprintln!("No theme installed");
        }
    }

    pub fn write_installed_theme(&self, theme_name: &str) {
        std::fs::create_dir_all(self.dir.join(".cache")).expect("Could not create cache directory");
        std::fs::write(self.dir.join(".cache/installed"), theme_name).expect("Could not record installed theme");
    }
}
