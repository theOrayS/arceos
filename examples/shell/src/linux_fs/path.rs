use std::string::{String, ToString};
use std::vec::Vec;

pub fn normalize_path(base: &str, path: &str) -> Option<String> {
    let mut parts = Vec::new();
    let input = if path.starts_with('/') {
        path.to_string()
    } else if base == "/" {
        format!("/{path}")
    } else {
        format!("{}/{}", base.trim_end_matches('/'), path)
    };
    for part in input.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            _ => parts.push(part),
        }
    }
    let mut normalized = String::from("/");
    normalized.push_str(&parts.join("/"));
    Some(normalized)
}

pub fn resolve_cwd_path(cwd: &str, path: &str) -> Option<String> {
    normalize_path(cwd, path)
}

#[cfg(test)]
mod tests {
    use super::{normalize_path, resolve_cwd_path};

    #[test]
    fn normalizes_absolute_components() {
        assert_eq!(normalize_path("/", "/a/./b/../c"), Some("/a/c".into()));
    }

    #[test]
    fn joins_relative_path_to_cwd() {
        assert_eq!(
            resolve_cwd_path("/tmp/test", "a/b"),
            Some("/tmp/test/a/b".into())
        );
    }

    #[test]
    fn parent_at_root_stays_at_root() {
        assert_eq!(normalize_path("/", "../../../a"), Some("/a".into()));
    }
}
