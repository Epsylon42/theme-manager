use std::{collections::HashMap, path::{Path, PathBuf}};
use serde::Deserialize;

use crate::prelude::*;
use crate::themes::ThemeDesc;
use crate::hooks::HookLauncher;

#[derive(Debug, Deserialize)]
pub struct FileDesc {
    #[serde(default)]
    pub name: Option<String>,
    pub path: PathBuf,
    pub target: String,
    #[serde(default)]
    pub template: bool,
}

impl FileDesc {
    pub fn get_name(&self) -> &str {
        self.name.as_ref()
            .map(String::as_str)
            .or_else(|| self.path.to_str())
            .unwrap()
    }
}

#[derive(Debug, Deserialize)]
pub struct InstallDesc {
    #[serde(skip)]
    pub dir: PathBuf,
    pub vars: HashMap<String, String>,
    #[serde(alias = "file")]
    pub files: Vec<FileDesc>,
}

impl InstallDesc {
    pub fn install(&self, theme: &ThemeDesc, global_hooks: HookLauncher) {
        let theme_hooks = theme.get_hook_launcher();
        global_hooks.run_preinstall().unwrap();
        theme_hooks.run_preinstall().unwrap();

        for file in &self.files {
            if file.template {
                self.install_template(theme, file);
            } else {
                self.install_copy(theme, file);
            }
        }

        global_hooks.run_postinstall().unwrap();
        theme_hooks.run_postinstall().unwrap();
    }

    pub fn install_empty(&self, global_hooks: HookLauncher) {
        self.install(&Default::default(), global_hooks);
    }

    fn install_template(&self, theme: &ThemeDesc, file: &FileDesc) {
        let path = self.resolve_path(theme, &file.path);
        let template = std::fs::read_to_string(self.dir.join(path)).unwrap();
        let template = mustache::compile_str(&template).unwrap();

        let empty_values = HashMap::new();
        let values = if let Some(theme_unit) = theme.units.get(file.get_name()) {
            &theme_unit.values
        } else {
            &empty_values
        };

        let result = template.render_to_string(values).unwrap();
        let target = self.resolve_target(&file.target);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&target, &result).unwrap();
    }

    fn install_copy(&self, theme: &ThemeDesc, file: &FileDesc) {
        let path = self.resolve_path(theme, &file.path);
        let target = self.resolve_target(&file.target);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::copy(path, target).unwrap();
    }

    fn resolve_path(&self, theme: &ThemeDesc, path: &Path) -> PathBuf {
        let theme_path = theme.dir.join(path);
        if std::fs::metadata(&theme_path).is_ok() {
            theme_path
        } else {
            self.dir.join(path)
        }
    }

    fn resolve_target(&self, target: &str) -> PathBuf {
        let target = mustache::compile_str(target)
            .unwrap()
            .render_to_string(&self.vars)
            .unwrap();

        PathBuf::from(target)
    }
}

pub fn read_from(dir: &Path) -> Result<InstallDesc, Error> {
    let s = std::fs::read_to_string(dir.join("install.toml"))
        .context("Could not read install.toml")?;
    let mut desc: InstallDesc = toml::de::from_str(&s)
        .context("install.toml parse error")?;

    desc.dir = dir.to_owned();

    Ok(desc)
}
