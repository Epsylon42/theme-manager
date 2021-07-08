use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy)]
pub enum ReadDirOptions {
    Files,
    Directories,
    Both,
}

impl ReadDirOptions {
    fn filter(&self, metadata: &std::fs::Metadata) -> bool {
        match self {
            ReadDirOptions::Files => metadata.is_file(),
            ReadDirOptions::Directories => metadata.is_dir(),
            ReadDirOptions::Both => true,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum ReadDirEntryType {
    File,
    Directory,
}

impl ReadDirEntryType {
    pub fn is_file(&self) -> bool {
        *self == ReadDirEntryType::File
    }

    pub fn is_dir(&self) -> bool {
        *self == ReadDirEntryType::Directory
    }
}

pub struct ReadDirEntry {
    pub file_name: String,
    pub path: PathBuf,
    pub entry_type: ReadDirEntryType,
}

pub fn read_dir(
    dir: &Path,
    options: ReadDirOptions,
) -> Result<impl Iterator<Item = ReadDirEntry>, std::io::Error> {
    let iter = std::fs::read_dir(dir)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let metadata = std::fs::metadata(&entry.path()).ok()?;
            Some((entry, metadata))
        })
        .filter(move |(_, metadata)| options.filter(&metadata))
        .map(|(entry, metadata)| ReadDirEntry {
            file_name: entry.file_name().to_str().unwrap().to_owned(),
            path: entry.path(),
            entry_type: if metadata.is_file() {
                ReadDirEntryType::File
            } else {
                ReadDirEntryType::Directory
            },
        });

    Ok(iter)
}
