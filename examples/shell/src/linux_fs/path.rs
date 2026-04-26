use axerrno::LinuxError;
use std::string::{String, ToString};
use std::vec::Vec;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResolveOptions {
    pub allow_empty: bool,
}

impl ResolveOptions {
    pub const fn default() -> Self {
        Self { allow_empty: false }
    }

    pub const fn allow_empty() -> Self {
        Self { allow_empty: true }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedPath {
    pub path: String,
    pub had_trailing_slash: bool,
}

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

pub fn resolve_at_path(
    cwd: &str,
    dirfd_base: Option<&str>,
    path: &str,
    options: ResolveOptions,
) -> Result<ResolvedPath, LinuxError> {
    if path.is_empty() {
        if options.allow_empty {
            return Ok(ResolvedPath {
                path: String::new(),
                had_trailing_slash: false,
            });
        }
        return Err(LinuxError::ENOENT);
    }

    let had_trailing_slash = path.len() > 1 && path.ends_with('/');
    let base = if path.starts_with('/') {
        "/"
    } else {
        dirfd_base.unwrap_or(cwd)
    };
    let Some(path) = normalize_path(base, path) else {
        return Err(LinuxError::EINVAL);
    };
    Ok(ResolvedPath {
        path,
        had_trailing_slash,
    })
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

    #[test]
    fn resolve_empty_path_requires_option() {
        assert_eq!(
            super::resolve_at_path("/", None, "", super::ResolveOptions::default()),
            Err(axerrno::LinuxError::ENOENT)
        );
        assert_eq!(
            super::resolve_at_path("/", None, "", super::ResolveOptions::allow_empty()),
            Ok(super::ResolvedPath {
                path: String::new(),
                had_trailing_slash: false,
            })
        );
    }

    #[test]
    fn resolve_relative_path_uses_dirfd_base_before_cwd() {
        assert_eq!(
            super::resolve_at_path(
                "/cwd",
                Some("/dirfd"),
                "child",
                super::ResolveOptions::default()
            ),
            Ok(super::ResolvedPath {
                path: "/dirfd/child".into(),
                had_trailing_slash: false,
            })
        );
    }

    #[test]
    fn resolve_absolute_path_ignores_dirfd_base() {
        assert_eq!(
            super::resolve_at_path(
                "/cwd",
                Some("/dirfd"),
                "/abs/file",
                super::ResolveOptions::default()
            ),
            Ok(super::ResolvedPath {
                path: "/abs/file".into(),
                had_trailing_slash: false,
            })
        );
    }

    #[test]
    fn resolve_records_trailing_slash() {
        assert_eq!(
            super::resolve_at_path("/", None, "tmp/", super::ResolveOptions::default()),
            Ok(super::ResolvedPath {
                path: "/tmp".into(),
                had_trailing_slash: true,
            })
        );
    }
}
