use std::{collections::HashMap, path::{Path, PathBuf}};

pub mod install;
pub mod themes;
pub mod error;
pub mod utils;
pub mod hooks;

mod prelude {
    pub use crate::error::{Error, ErrorExt, ResultExt};
    pub use crate::utils;
}

use prelude::*;

use install::InstallDesc;
use themes::ThemeDesc;

#[derive(Debug)]
pub struct ThemeManager {
    dir: PathBuf,
    install: InstallDesc,
    themes: HashMap<String, ThemeDesc>,
}

impl ThemeManager {
    pub fn read_from_dir(dir: &Path) -> Result<Self, Error> {
        Ok(ThemeManager {
            dir: dir.to_owned(),
            install: install::read_install(&dir.join("install"))
                .context("Could not read install directory")?,
            themes: themes::read_themes(dir),
        })
    }

    pub fn install_theme(&self, theme: &str) {
        let theme = &self.themes[theme];
        self.install.install(theme, hooks::HookLauncher::Empty);
    }

    pub fn install_empty(&self) {
        self.install.install_empty(hooks::HookLauncher::Empty);
    }
}

fn main() {
    match run() {
        Ok(_) => {},
        Err(e) => eprintln!("{}", e)
    }
}

fn run() -> Result<(), Error> {
    let data = std::env::args().nth(1).unwrap();
    let theme_name = std::env::args().nth(2).unwrap();

    let manager = ThemeManager::read_from_dir(Path::new(&data))?;
    manager.install_theme(&theme_name);
    Ok(())
}
