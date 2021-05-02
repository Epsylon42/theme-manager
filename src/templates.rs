use std::{collections::HashMap, path::PathBuf};
use serde::Deserialize;

use crate::prelude::*;

#[derive(Debug, Deserialize)]
pub struct TemplateFileDesc {
    pub file: PathBuf,
    pub target: String,
}

impl TemplateFileDesc {
    pub fn get_name(&self) -> &str {
        self.file.to_str().unwrap()
    }
}

#[derive(Debug, Deserialize)]
pub struct TemplatesDesc {
    pub vars: HashMap<String, String>,
    #[serde(alias = "file")]
    pub units: Vec<TemplateFileDesc>,
}

impl TemplatesDesc {
    pub fn resolve_target(&self, target: &str) -> Result<PathBuf, Error> {
        let result = mustache::compile_str(target)
            .unwrap()
            .render_to_string(&self.vars)
            .unwrap();

        Ok(PathBuf::from(result))
    }
}
