use std::path::PathBuf;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TemplateFileDesc {
    pub file: PathBuf,
    pub target: PathBuf,
}

impl TemplateFileDesc {
    pub fn get_name(&self) -> &str {
        self.file.to_str().unwrap()
    }
}

#[derive(Debug, Deserialize)]
pub struct TemplatesDesc {
    #[serde(rename = "file")]
    pub units: Vec<TemplateFileDesc>,
}
