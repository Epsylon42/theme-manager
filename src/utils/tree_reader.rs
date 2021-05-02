use std::path::{Path, PathBuf};

use crate::prelude::*;

#[derive(Debug, PartialEq)]
pub enum TreeReaderNode {
    Literal(String),
    Any,
}

#[derive(Debug)]
pub struct TreeReader<'a> {
    dir: &'a Path,
    desc: &'a[TreeReaderNode],
}

impl<'a> TreeReader<'a> {
    pub fn new(dir: &'a Path, desc: &'a[TreeReaderNode]) -> Self {
        TreeReader {
            dir,
            desc,
        }
    }

    pub fn get_file_entries_recursive(&self) -> Vec<TreeReaderEntry> {
        let add_additional_capture = self.desc.get(0) == Some(&TreeReaderNode::Any);

        let mut entries = self.get_file_entries();
        for dir_entry in self.get_dir_entries() {
            if let Some(reader) = self.step_down(&dir_entry.path) {
                let mut new_entries = reader.get_file_entries_recursive();
                if add_additional_capture {
                    let additional_capture = reader.dir.file_name()
                        .and_then(|file_name| file_name.to_str())
                        .unwrap_or("");

                    for entry in &mut new_entries {
                        entry.captures.0.insert(0, String::from(additional_capture));
                    }
                }

                entries.extend(new_entries);
            }
        }

        entries
    }

    pub fn get_file_entries(&self) -> Vec<TreeReaderEntry> {
        utils::read_dir(self.dir, utils::ReadDirOptions::Files)
            .map(|read_dir| {
                read_dir.filter_map(|entry| {
                    match_file_name(self.desc, &entry.file_name)
                        .map(|captures| TreeReaderEntry { path: entry.path, captures })
                })
                .collect()
            })
            .unwrap_or_else(|_| Vec::new())
    }

    pub fn get_dir_entries(&self) -> Vec<TreeReaderEntry> {
        utils::read_dir(self.dir, utils::ReadDirOptions::Directories)
            .map(|read_dir| {
                read_dir.filter_map(|entry| {
                    match_dir_name(self.desc, &entry.file_name)
                        .map(|captures| TreeReaderEntry { path: entry.path, captures })
                })
                .collect()
            })
            .unwrap_or_else(|_| Vec::new())
    }

    fn step_down(&self, dir: &'a Path) -> Option<Self> {
        Some(TreeReader {
            dir,
            desc: self.desc.get(1..)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct TreeReaderEntry {
    pub path: PathBuf,
    pub captures: Captures,
}

#[derive(Debug, Clone)]
pub struct Captures(pub Vec<String>);

fn match_file_name(desc: &[TreeReaderNode], name: &str) -> Option<Captures> {
    if name.split('-').count() != desc.len() {
        return None;
    }

    let mut captures = Vec::new();
    for (desc_part, name_part) in desc.iter().zip(name.split('-')) {
        match desc_part {
            TreeReaderNode::Literal(x) => if x != name_part {
                return None;
            }

            TreeReaderNode::Any => {
                captures.push(String::from(name_part));
            }
        }
    }

    Some(Captures(captures))
}

fn match_dir_name(desc: &[TreeReaderNode], name: &str) -> Option<Captures> {
    if name.split('-').count() > desc.len() {
        return None;
    }

    let name_parts = name.split('-').collect::<Vec<_>>();

    let last_name_part = &name_parts[name_parts.len()-1];
    let last_relevant_desc = &desc[name_parts.len()-1];
    match last_relevant_desc {
        TreeReaderNode::Literal(x) => if last_name_part != &make_plural(x) {
            return None;
        }

        TreeReaderNode::Any => if name_parts.len() == desc.len() {
            return None;
        }
    }

    let mut captures = Vec::new();
    let name_parts_except_last = &name_parts[..name_parts.len()-1];
    for (desc_part, name_part) in desc.iter().zip(name_parts_except_last) {
        match desc_part {
            TreeReaderNode::Literal(x) => if x != name_part {
                return None;
            }

            TreeReaderNode::Any => {
                captures.push(String::from(*name_part));
            }
        }
    }

    Some(Captures(captures))
}

fn make_plural(s: impl Into<String>) -> String {
    s.into() + "s"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_empty() {
        assert!(match_file_name(&[], "abc").is_none());
        assert!(match_dir_name(&[], "abc").is_none());
    }

    #[test]
    fn match_any_file() {
        let captures = match_file_name(&[
            TreeReaderNode::Any
        ], "abc");
        assert!(captures.is_some());
        assert_eq!(captures.unwrap().0, &[String::from("abc")]);
    }

    #[test]
    fn match_exact_file() {
        let captures = match_file_name(&[
            TreeReaderNode::Literal(String::from("abc"))
        ], "abc");
        assert!(captures.is_some());
        let expected: &[String] = &[];
        assert_eq!(captures.unwrap().0, expected);

        let captures = match_file_name(&[
            TreeReaderNode::Literal(String::from("def"))
        ], "abc");
        assert!(captures.is_none());
    }

    #[test]
    fn match_any_dir() {
        let captures = match_dir_name(&[
            TreeReaderNode::Any
        ], "abc");
        assert!(captures.is_none());

        let captures = match_dir_name(&[
            TreeReaderNode::Any
        ], "abc-def");
        assert!(captures.is_none());
    }

    #[test]
    fn match_leaf_dir() {
        let captures = match_dir_name(&[
            TreeReaderNode::Literal(String::from("theme")),
            TreeReaderNode::Any,
        ], "themes");
        assert!(captures.is_some());
        let expected: &[String] = &[];
        assert_eq!(captures.unwrap().0, expected);
    }

    #[test]
    fn match_units() {
        let nodes = &[
            TreeReaderNode::Literal(String::from("theme")),
            TreeReaderNode::Any,
            TreeReaderNode::Literal(String::from("unit")),
            TreeReaderNode::Any,
            TreeReaderNode::Any,
        ];

        let captures = match_file_name(nodes, "theme-dark-unit-termite-color");
        assert!(captures.is_some());
        assert_eq!(captures.unwrap().0, &[
            String::from("dark"),
            String::from("termite"),
            String::from("color"),
        ]);

        assert!(match_dir_name(nodes, "themes").is_some());
        assert!(match_dir_name(nodes, "theme-dark-units").is_some());
        assert!(match_dir_name(nodes, "theme-dark-unit-termite").is_some());
        assert!(match_dir_name(nodes, "theme-dark-unit-termite-color").is_none());
    }
}
