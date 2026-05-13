use std::collections::HashMap;

use crate::{cmd_parser::is_arceos_crate, parse_deps};

/// without further exploiting the feature of d2 graph, this is almost the same syntax with mermaid
/// except that d2 use " -> ", instead of "-->"
pub fn gen_d2_script(deps: &String, result: &mut String) {
    let deps_parsed = parse_deps(&deps);
    if deps_parsed.is_empty() {
        return;
    }
    let dep_root = &deps_parsed[0];

    let mut parsed_crates: Vec<&String> = Vec::new();
    let mut lastest_dep_map: HashMap<i32, &String> = HashMap::new();
    let mut idx: usize = 1;

    lastest_dep_map.insert(0, &dep_root.1);
    while idx < deps_parsed.len() {
        let (level, name) = deps_parsed.get(idx).unwrap();
        if !is_arceos_crate(&name) {
            idx += 1;
            continue;
        }
        if let Some(parent) = nearest_arceos_parent(&lastest_dep_map, *level) {
            *result += &format!("{} -> {}\n", parent, name);
        }
        if parsed_crates.contains(&name) {
            let mut skip_idx: usize = idx + 1;
            if skip_idx >= deps_parsed.len() {
                break;
            }
            while let Some(next_dep) = deps_parsed.get(skip_idx) {
                if next_dep.0 <= *level {
                    break;
                }
                idx += 1;
                skip_idx += 1;
            }
            idx += 1;
        } else {
            parsed_crates.push(&name);
            lastest_dep_map.insert(*level, name);
            idx += 1;
        }
    }
}

fn nearest_arceos_parent<'a>(
    deps_by_level: &HashMap<i32, &'a String>,
    level: i32,
) -> Option<&'a String> {
    (0..level)
        .rev()
        .find_map(|parent_level| deps_by_level.get(&parent_level).copied())
}
