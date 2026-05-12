use std::string::{String, ToString};
use std::vec::Vec;

use super::linux_abi::TESTSUITE_STAGE_ROOT;

pub(super) fn current_cwd() -> String {
    std::env::current_dir().unwrap_or_else(|_| "/".into())
}

pub(super) fn resolve_host_path(cwd: String, path: &str) -> Result<String, String> {
    normalize_path(cwd.as_str(), path).ok_or_else(|| format!("invalid path: {path}"))
}

pub(super) fn busybox_applet_target_path(path: &str) -> Option<String> {
    let (root, applet) = path
        .strip_prefix("/musl/")
        .map(|applet| ("/musl", applet))
        .or_else(|| {
            path.strip_prefix("/glibc/")
                .map(|applet| ("/glibc", applet))
        })?;
    if applet.is_empty()
        || applet.contains('/')
        || applet == "busybox"
        || !is_busybox_applet_name(applet)
    {
        return None;
    }
    Some(format!("{root}/busybox"))
}

pub(super) fn normalize_path(base: &str, path: &str) -> Option<String> {
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

fn is_busybox_applet_name(name: &str) -> bool {
    matches!(
        name,
        "[" | "ash"
            | "basename"
            | "cal"
            | "cat"
            | "clear"
            | "cp"
            | "cut"
            | "date"
            | "df"
            | "dirname"
            | "dmesg"
            | "du"
            | "echo"
            | "expr"
            | "false"
            | "find"
            | "free"
            | "grep"
            | "head"
            | "hexdump"
            | "hwclock"
            | "kill"
            | "ls"
            | "md5sum"
            | "mkdir"
            | "more"
            | "mv"
            | "od"
            | "printf"
            | "ps"
            | "pwd"
            | "rm"
            | "rmdir"
            | "sh"
            | "sleep"
            | "sort"
            | "stat"
            | "strings"
            | "tail"
            | "test"
            | "touch"
            | "true"
            | "uname"
            | "uniq"
            | "uptime"
            | "wc"
            | "which"
    )
}

pub(super) fn derive_exec_root_from_path(path: &str) -> String {
    if path == "/musl" || path.starts_with("/musl/") {
        return "/musl".into();
    }
    if path == "/glibc" || path.starts_with("/glibc/") {
        return "/glibc".into();
    }
    if path.starts_with(TESTSUITE_STAGE_ROOT) {
        let Some(rest) = path.strip_prefix(TESTSUITE_STAGE_ROOT) else {
            return "/".into();
        };
        if rest == "/musl" || rest.starts_with("/musl/") {
            return "/musl".into();
        }
        if rest == "/glibc" || rest.starts_with("/glibc/") {
            return "/glibc".into();
        }
    }
    "/".into()
}

pub(super) fn resolve_runtime_support_file(exec_root: &str, path: &str) -> Result<String, String> {
    let candidates = if path.starts_with('/') {
        runtime_absolute_path_candidates(exec_root, path)
    } else if !path.contains('/') {
        runtime_library_name_candidates(exec_root, path)
    } else {
        vec![normalize_path("/", path).ok_or_else(|| format!("invalid path: {path}"))?]
    };
    candidates
        .into_iter()
        .find(|candidate| matches!(std::fs::metadata(candidate), Ok(meta) if meta.is_file()))
        .ok_or_else(|| format!("runtime support file not found: {path}"))
}

pub(super) fn runtime_absolute_path_candidates(exec_root: &str, path: &str) -> Vec<String> {
    let Some(normalized) = normalize_path("/", path) else {
        return Vec::new();
    };
    let mut candidates = vec![normalized.clone()];
    for root in runtime_root_candidates(exec_root, normalized.as_str()) {
        if normalized == "/lib" || normalized.starts_with("/lib/") {
            push_runtime_candidate(
                &mut candidates,
                join_runtime_root(root.as_str(), normalized.as_str()),
            );
            if normalized == "/lib" {
                push_runtime_candidate(&mut candidates, join_runtime_root(root.as_str(), "/lib64"));
            } else if let Some(suffix) = normalized.strip_prefix("/lib/") {
                push_runtime_candidate(
                    &mut candidates,
                    join_runtime_root(root.as_str(), format!("/lib64/{suffix}").as_str()),
                );
                push_multiarch_runtime_aliases(&mut candidates, root.as_str(), suffix);
            }
        } else if normalized == "/lib64" || normalized.starts_with("/lib64/") {
            push_runtime_candidate(
                &mut candidates,
                join_runtime_root(root.as_str(), normalized.as_str()),
            );
            if normalized == "/lib64" {
                push_runtime_candidate(&mut candidates, join_runtime_root(root.as_str(), "/lib"));
            } else if let Some(suffix) = normalized.strip_prefix("/lib64/") {
                push_runtime_candidate(
                    &mut candidates,
                    join_runtime_root(root.as_str(), format!("/lib/{suffix}").as_str()),
                );
                push_multiarch_runtime_aliases(&mut candidates, root.as_str(), suffix);
            }
        } else if normalized == "/usr/lib" || normalized.starts_with("/usr/lib/") {
            push_runtime_candidate(
                &mut candidates,
                join_runtime_root(root.as_str(), normalized.as_str()),
            );
            if normalized == "/usr/lib" {
                push_runtime_candidate(&mut candidates, join_runtime_root(root.as_str(), "/lib"));
                push_runtime_candidate(&mut candidates, join_runtime_root(root.as_str(), "/lib64"));
            } else if let Some(suffix) = normalized.strip_prefix("/usr/lib/") {
                push_runtime_candidate(
                    &mut candidates,
                    join_runtime_root(root.as_str(), format!("/lib/{suffix}").as_str()),
                );
                push_runtime_candidate(
                    &mut candidates,
                    join_runtime_root(root.as_str(), format!("/lib64/{suffix}").as_str()),
                );
                push_multiarch_runtime_aliases(&mut candidates, root.as_str(), suffix);
            }
        } else if normalized == "/usr/lib64" || normalized.starts_with("/usr/lib64/") {
            push_runtime_candidate(
                &mut candidates,
                join_runtime_root(root.as_str(), normalized.as_str()),
            );
            if normalized == "/usr/lib64" {
                push_runtime_candidate(&mut candidates, join_runtime_root(root.as_str(), "/lib64"));
                push_runtime_candidate(&mut candidates, join_runtime_root(root.as_str(), "/lib"));
            } else if let Some(suffix) = normalized.strip_prefix("/usr/lib64/") {
                push_runtime_candidate(
                    &mut candidates,
                    join_runtime_root(root.as_str(), format!("/lib64/{suffix}").as_str()),
                );
                push_runtime_candidate(
                    &mut candidates,
                    join_runtime_root(root.as_str(), format!("/lib/{suffix}").as_str()),
                );
                push_multiarch_runtime_aliases(&mut candidates, root.as_str(), suffix);
            }
        } else if normalized.starts_with("/etc/ld") {
            push_runtime_candidate(
                &mut candidates,
                join_runtime_root(root.as_str(), normalized.as_str()),
            );
        }
        push_musl_loader_aliases(&mut candidates, root.as_str(), normalized.as_str());
    }
    candidates
}

pub(super) fn runtime_library_name_candidates(exec_root: &str, name: &str) -> Vec<String> {
    if name.contains('/') || !looks_like_runtime_library_name(name) {
        return Vec::new();
    }
    let mut candidates = Vec::new();
    for root in runtime_root_candidates(exec_root, name) {
        push_runtime_candidate(
            &mut candidates,
            join_runtime_root(root.as_str(), format!("/lib/{name}").as_str()),
        );
        push_runtime_candidate(
            &mut candidates,
            join_runtime_root(root.as_str(), format!("/lib64/{name}").as_str()),
        );
        push_runtime_candidate(
            &mut candidates,
            join_runtime_root(root.as_str(), format!("/usr/lib/{name}").as_str()),
        );
        push_runtime_candidate(
            &mut candidates,
            join_runtime_root(root.as_str(), format!("/usr/lib64/{name}").as_str()),
        );
        push_musl_loader_aliases(&mut candidates, root.as_str(), name);
    }
    candidates
}

fn runtime_root_candidates(exec_root: &str, path: &str) -> Vec<String> {
    let name = path.rsplit('/').next().unwrap_or(path);
    let mut roots = Vec::new();
    let mut push = |root: &str| {
        if !roots.iter().any(|item| item == root) {
            roots.push(root.to_string());
        }
    };
    if is_glibc_runtime_name(name) {
        push("/glibc");
    }
    if is_musl_runtime_name(name) {
        push("/musl");
    }
    if exec_root != "/" {
        push(exec_root);
    }
    push("/musl");
    push("/glibc");
    roots
}

fn join_runtime_root(root: &str, path: &str) -> Option<String> {
    let normalized = normalize_path("/", path)?;
    if root == "/" {
        return Some(normalized);
    }
    let rel = normalized.trim_start_matches('/');
    Some(if rel.is_empty() {
        root.to_string()
    } else {
        format!("{}/{}", root.trim_end_matches('/'), rel)
    })
}

pub(super) fn push_runtime_candidate(candidates: &mut Vec<String>, candidate: Option<String>) {
    let Some(candidate) = candidate else {
        return;
    };
    if !candidates.iter().any(|item| item == &candidate) {
        candidates.push(candidate);
    }
}

fn push_multiarch_runtime_aliases(candidates: &mut Vec<String>, root: &str, suffix: &str) {
    let Some((_, tail)) = suffix.split_once('/') else {
        return;
    };
    if tail.is_empty() {
        return;
    }
    push_runtime_candidate(
        candidates,
        join_runtime_root(root, format!("/lib/{tail}").as_str()),
    );
    push_runtime_candidate(
        candidates,
        join_runtime_root(root, format!("/lib64/{tail}").as_str()),
    );
}

fn push_musl_loader_aliases(candidates: &mut Vec<String>, root: &str, path: &str) {
    let name = path.rsplit('/').next().unwrap_or(path);
    if !name.starts_with("ld-musl-") || !name.ends_with(".so.1") {
        return;
    }
    push_runtime_candidate(candidates, join_runtime_root(root, "/lib/libc.so"));
    push_runtime_candidate(candidates, join_runtime_root(root, "/lib64/libc.so"));
}

fn is_glibc_runtime_name(name: &str) -> bool {
    name.starts_with("ld-linux-") || name.ends_with(".so.6")
}

fn is_musl_runtime_name(name: &str) -> bool {
    name.starts_with("ld-musl-") || name == "libc.so"
}

fn looks_like_runtime_library_name(name: &str) -> bool {
    name.starts_with("ld-") || name.contains(".so")
}
