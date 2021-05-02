use std::{collections::HashMap, path::{Path, PathBuf}};

pub mod templates;
pub mod themes;
pub mod error;
pub mod utils;

mod prelude {
    pub use crate::error::{Error, ErrorExt, ResultExt};
    pub use crate::utils;
}

use prelude::*;

use templates::TemplatesDesc;
use themes::ThemeDesc;

#[derive(Debug)]
pub struct ThemeManager {
    dir: PathBuf,
    templates: TemplatesDesc,
    themes: HashMap<String, ThemeDesc>,
}

impl ThemeManager {
    pub fn read_from_dir(dir: &Path) -> Result<Self, Error> {
        Ok(ThemeManager {
            dir: dir.to_owned(),
            templates: Self::read_templates(&dir.join("templates"))
                .context("Could not read templates directory")?,
            themes: Self::read_themes(&dir.join("themes"))
                .context("Could not read themes")?,
        })
    }

    fn read_templates(dir: &Path) -> Result<TemplatesDesc, Error> {
        let s = std::fs::read_to_string(dir.join("templates.toml"))
            .context("Could not read templates.toml")?;
        let templates_desc: TemplatesDesc = toml::de::from_str(&s)
            .context("templates.toml parse error")?;
        Ok(templates_desc)
    }

    fn read_themes(dir: &Path) -> Result<HashMap<String, ThemeDesc>, Error> {
        let themes = utils::read_dir(dir, utils::ReadDirOptions::Directories).unwrap()
            .map(|entry| {
                ThemeDesc::read_from_dir(&entry.path)
                    .map(|theme| (entry.file_name, theme))
            })
            .collect::<Result<_, Error>>().unwrap();

        Ok(themes)
    }

    pub fn install_theme(&self, theme: &str) {
        let theme = &self.themes[theme];
        let empty_values = HashMap::new();

        for unit in &self.templates.units {
            let template = std::fs::read_to_string(self.dir.join("templates").join(&unit.file)).unwrap();
            let template = mustache::compile_str(&template).unwrap();

            let values = if let Some(theme_unit) = theme.units.get(unit.get_name()) {
                &theme_unit.values
            } else {
                &empty_values
            };
            let result = template.render_to_string(values).unwrap();

            let target = self.templates.resolve_target(&unit.target).unwrap();
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(&target, &result).unwrap();
        }
    }

    pub fn install_empty(&self) {
        for unit in &self.templates.units {
            let template = std::fs::read_to_string(self.dir.join("templates").join(&unit.file)).unwrap();

            let target = self.templates.resolve_target(&unit.target).unwrap();
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(&target, &template).unwrap();
        }
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
