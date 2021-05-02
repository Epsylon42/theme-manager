use std::path::Path;
use std::collections::{hash_map, HashMap};
use serde::Deserialize;

use crate::prelude::*;
use utils::tree_reader::{TreeReader, TreeReaderNode};

#[derive(Debug, Default, Deserialize)]
pub struct UnitDesc {
    pub values: HashMap<String, String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct ThemeDesc {
    pub hooks: (),
    #[serde(alias = "unit")]
    pub units: HashMap<String, UnitDesc>,
}

pub fn read_themes(dir: &Path) -> HashMap<String, ThemeDesc> {
    let desc = &[
        TreeReaderNode::Literal(String::from("theme")),
        TreeReaderNode::Any,
        TreeReaderNode::Literal(String::from("unit")),
        TreeReaderNode::Any,
        TreeReaderNode::Any,
    ];

    let mut themes = HashMap::<String, ThemeDesc>::new();
    for entry in TreeReader::new(dir, desc).get_file_entries_recursive() {
        assert_eq!(entry.captures.0.len(), 3);
        let mut captures = entry.captures.0;

        let value = std::fs::read_to_string(entry.path).unwrap();

        let value_name = captures.pop().unwrap();
        let unit_name = captures.pop().unwrap();
        let theme_name = captures.pop().unwrap();

        let theme = ensure_contains(&mut themes, theme_name);
        let unit = ensure_contains(&mut theme.units, unit_name);
        unit.values.insert(value_name, value);
    }

    themes
}

fn ensure_contains<'a, T: Default>(map: &'a mut HashMap<String, T>, key: String) -> &'a mut T {
    match map.entry(key) {
        hash_map::Entry::Vacant(entry) => entry.insert(T::default()),
        hash_map::Entry::Occupied(entry) => entry.into_mut(),
    }
}
