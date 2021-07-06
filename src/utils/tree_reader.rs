use std::path::{Path, PathBuf};

use crate::prelude::*;

#[derive(Debug)]
pub enum TreeReaderNode {
    Literal(String),
    Pattern(regex::Regex),
    AnyDir,
    Any,
}

#[derive(Debug)]
pub struct TreeReader<'a> {
    dir: &'a Path,
    desc: &'a[TreeReaderNode],
}

impl<'a> TreeReader<'a> {
    pub fn new(dir: &'a Path, desc: &'a[TreeReaderNode]) -> Self {
        assert!(desc.len() > 0);

        TreeReader {
            dir,
            desc,
        }
    }

    pub fn get_file_entries_recursive(&self) -> Vec<TreeReaderEntry> {
        let mut entries = self.get_file_entries();
        for dir_entry in self.get_dir_entries() {
            if let Some(reader) = self.step_down(&dir_entry.path) {
                let mut new_entries = reader.get_file_entries_recursive();
                for entry in &mut new_entries {
                    entry.captures.0.splice(..0, dir_entry.captures.0.iter().cloned());
                }

                entries.extend(new_entries);
            }
        }

        debug_assert!(entries.iter().all(|entry| entry.captures.0.len() == self.expected_num_captures()));

        entries
    }

    pub fn get_dir_entries_recursive(&self) -> Vec<TreeReaderEntry> {
        let mut entries = self.get_dir_entries()
            .into_iter()
            .filter(|entry| entry.captures.0.len() == self.expected_num_captures())
            .collect::<Vec<_>>();

        for dir_entry in self.get_dir_entries() {
            if let Some(reader) = self.step_down(&dir_entry.path) {
                let new_entries = reader.get_dir_entries_recursive()
                    .into_iter()
                    .filter(|entry| entry.captures.0.len() == self.expected_num_captures())
                    .map(|mut entry| {
                        entry.captures.0.splice(..0, dir_entry.captures.0.iter().cloned());
                        entry
                    });

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
            desc: self.desc.get(dir.file_name()?.to_str()?.split('-').count()..)?,
        })
    }

    fn expected_num_captures(&self) -> usize {
        self.desc.iter()
            .map(|node| match node {
                TreeReaderNode::Any | TreeReaderNode::AnyDir => 1,
                TreeReaderNode::Pattern(pat) => pat.captures_len() - 1,
                _ => 0,
            })
            .sum()
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
    if name.starts_with('_') {
        return None;
    }

    if name.split('-').count() != desc.len() {
        return None;
    }

    let mut captures = Vec::new();
    for (desc_part, name_part) in desc.iter().zip(name.split('-')) {
        match desc_part {
            TreeReaderNode::Literal(x) => if x != name_part {
                return None;
            }

            TreeReaderNode::Pattern(pat) => if let Some(cap) = pat.captures(name_part) {
                for i in 1..cap.len() {
                    if let Some(cap) = cap.get(i) {
                        captures.push(cap.as_str().to_owned());
                    }
                }
            } else {
                return None;
            }

            TreeReaderNode::AnyDir => {
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
    if name.starts_with('_') {
        return None;
    }

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

        TreeReaderNode::Any | TreeReaderNode::Pattern(_) => if name_parts.len() == desc.len() {
            return None;
        }

        TreeReaderNode::AnyDir => {}
    }

    let mut captures = Vec::new();
    let name_parts_except_last = &name_parts[..name_parts.len()-1];
    for (desc_part, name_part) in desc.iter().zip(name_parts_except_last) {
        match desc_part {
            TreeReaderNode::Literal(x) => if x != name_part {
                return None;
            }

            TreeReaderNode::Pattern(pat) => if let Some(cap) = pat.captures(name_part) {
                for i in 1..cap.len() {
                    if let Some(cap) = cap.get(i) {
                        captures.push(cap.as_str().to_owned());
                    }
                }
            } else {
                return None;
            }

            TreeReaderNode::Any | TreeReaderNode::AnyDir => {
                captures.push(String::from(*name_part));
            }
        }
    }

    if matches!(last_relevant_desc, TreeReaderNode::Any | TreeReaderNode::AnyDir) {
        captures.push(String::from(*last_name_part));
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
    fn match_anydir_dir() {
        let captures = match_dir_name(&[
            TreeReaderNode::AnyDir
        ], "abc");
        assert!(captures.is_some());
        assert_eq!(captures.unwrap().0, &[String::from("abc")]);

        let captures = match_dir_name(&[
            TreeReaderNode::AnyDir
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
    fn match_theme_directory() {
        let nodes = &[
            TreeReaderNode::Literal(String::from("theme")),
            TreeReaderNode::AnyDir,
            TreeReaderNode::Literal(String::from("unit")),
            TreeReaderNode::Any,
            TreeReaderNode::Any,
        ];

        assert!(match_dir_name(nodes, "themes").is_some());

        let captures = match_dir_name(nodes, "theme-dark");
        assert!(captures.is_some());
        assert_eq!(captures.unwrap().0, &[String::from("dark")]);

        let captures = match_file_name(nodes, "theme-dark-unit-termite-color");
        assert!(captures.is_none());
    }

    #[test]
    fn match_units() {
        let nodes = &[
            TreeReaderNode::Literal(String::from("unit")),
            TreeReaderNode::Any,
            TreeReaderNode::Any,
        ];

        let captures = match_file_name(nodes, "unit-termite-color");
        assert!(captures.is_some());
        assert_eq!(captures.unwrap().0, &[String::from("termite"), String::from("color")]);

        assert!(match_file_name(nodes, "unit-abc").is_none());

        assert!(match_dir_name(nodes, "units").is_some());
        assert!(match_dir_name(nodes, "unit-termite").is_some());
        assert!(match_dir_name(nodes, "unit-termite-color").is_none());
    }

    #[test]
    fn match_pattern() {
        let nodes = &[
            TreeReaderNode::Literal(String::from("unit")),
            TreeReaderNode::Pattern(regex::Regex::new("^(.*)\\.toml$").unwrap())
        ];

        let captures = match_file_name(nodes, "unit-test.toml");
        assert!(captures.is_some());
        assert_eq!(captures.unwrap().0, &[String::from("test")]);
    }

    #[test]
    fn ignore() {
        let captures = match_file_name(&[
            TreeReaderNode::Any
        ], "_abc");
        assert!(captures.is_none());

        let captures = match_dir_name(&[
            TreeReaderNode::Literal(String::from("unit")),
            TreeReaderNode::Any,
        ], "_units");
        assert!(captures.is_none());
    }
}
