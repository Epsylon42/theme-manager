use serde::Deserialize;
use std::{
    collections::HashMap,
    convert::TryFrom,
    path::{Path, PathBuf},
};

use crate::hooks::HookLauncher;
use crate::prelude::*;
use crate::themes::ThemeDesc;

#[derive(Debug, Deserialize)]
pub struct FileDescDeserialize {
    #[serde(default)]
    pub name: Option<String>,
    pub path: PathBuf,
    pub target: String,
    #[serde(default)]
    pub template: bool,
}

#[derive(Debug, Deserialize)]
#[serde(try_from = "FileDescDeserialize")]
pub struct FileDesc {
    pub name: String,
    pub path: PathBuf,
    pub target: String,
    pub template: bool,
}

impl TryFrom<FileDescDeserialize> for FileDesc {
    type Error = Error;

    fn try_from(value: FileDescDeserialize) -> Result<Self, Self::Error> {
        let file_stem = value
            .path
            .file_stem()
            .ok_or(Error::InvalidPath { /*TODO*/ })?
            .to_str()
            .ok_or(Error::InvalidPath { /*TODO*/ })?;

        let name = value.name.unwrap_or_else(|| String::from(file_stem));

        Ok(FileDesc {
            name,
            path: value.path,
            target: value.target,
            template: value.template,
        })
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
    pub fn install(
        &self,
        theme_chain: &[&ThemeDesc],
        global_hooks: HookLauncher,
    ) -> Result<(), Error> {
        assert!(!theme_chain.is_empty());
        trace!("Installing theme '{}'", theme_chain.last().unwrap().name);
        for inherited in theme_chain.into_iter().rev().skip(1) {
            trace!("Inherits '{}'", inherited.name);
        }

        global_hooks
            .run_preinstall()
            .context("Global preinstall hooks")?;
        for theme in theme_chain {
            theme
                .get_hook_launcher()
                .run_preinstall()
                .with_context(|| format!("Theme '{}' preinstall hook", theme.name))?;
        }

        for file in &self.files {
            let res = if file.template {
                self.install_template(theme_chain, file)
            } else {
                self.install_copy(theme_chain, file)
            };

            res.with_context(|| format!("Installing {}", file.name))?;
        }

        global_hooks
            .run_postinstall()
            .context("Global postinstall hooks")?;
        for theme in theme_chain {
            theme
                .get_hook_launcher()
                .run_postinstall()
                .with_context(|| format!("Theme '{}' preinstall hook", theme.name))?;
        }

        Ok(())
    }

    pub fn install_empty(&self, global_hooks: HookLauncher) -> Result<(), Error> {
        self.install(&[&Default::default()], global_hooks)
    }

    fn install_template(&self, theme_chain: &[&ThemeDesc], unit: &FileDesc) -> Result<(), Error> {
        trace!("Installing template '{}'", unit.name);

        let path = self.resolve_theme_chain_path(theme_chain, &unit.path);

        let template =
            std::fs::read_to_string(self.dir.join(path)).context("Failed to read template file")?;
        let template =
            mustache::compile_str(&template).context("Failed to compile mustache template")?;

        let mut values = HashMap::new();
        for theme in theme_chain {
            if let Some(theme_unit) = theme.units.get(&unit.name) {
                values.extend(&theme_unit.values);
            }
        }

        let result = template.render_to_string(&values).unwrap();
        let target = self
            .resolve_target(&unit.target)
            .context("Failed to resolve installation path")?;
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).context("Failed to create parent directory")?;
        }
        std::fs::write(&target, &result).context("Failed to write file")?;

        Ok(())
    }

    fn install_copy(&self, theme_chain: &[&ThemeDesc], unit: &FileDesc) -> Result<(), Error> {
        trace!("Installing file '{}'", unit.name);

        let path = self.resolve_theme_chain_path(theme_chain, &unit.path);

        let target = self
            .resolve_target(&unit.target)
            .context("Failed to resolve installation path")?;
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::copy(path, target).unwrap();

        Ok(())
    }

    //fn resolve_theme_path(&self, theme: &ThemeDesc, path: &Path) -> Option<PathBuf> {
    //let theme_path = theme.dir.join(path);
    //if theme_path.exists() {
    //Some(theme_path)
    //} else {
    //None
    //}
    //}

    //fn resolve_theme_path_or_default(&self, theme: &ThemeDesc, path: &Path) -> PathBuf {
    //self.resolve_theme_path(theme, path).unwrap_or_else(|| self.dir.join(path))
    //}

    fn resolve_theme_chain_path(&self, theme_chain: &[&ThemeDesc], path: &Path) -> PathBuf {
        theme_chain
            .into_iter()
            .rev()
            .map(|theme| theme.dir.join(path))
            .find(|path| path.exists())
            .unwrap_or_else(|| self.dir.join(path))
    }

    fn resolve_target(&self, target: &str) -> Result<PathBuf, Error> {
        let target = mustache::compile_str(target)
            .unwrap()
            .render_to_string(&self.vars)
            .unwrap();

        Ok(PathBuf::from(target))
    }
}

pub fn read_from(dir: &Path) -> Result<InstallDesc, Error> {
    trace!("Reading install data from {:?}", dir);

    let s =
        std::fs::read_to_string(dir.join("install.toml")).context("Could not read install.toml")?;
    let mut desc: InstallDesc = toml::de::from_str(&s).context("install.toml parse error")?;

    desc.dir = dir.to_owned();

    Ok(desc)
}
