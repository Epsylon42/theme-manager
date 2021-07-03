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
        let theme = &self.themes[theme];
        self.install.install(theme, hooks::HookLauncher::HookSet {
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
}
